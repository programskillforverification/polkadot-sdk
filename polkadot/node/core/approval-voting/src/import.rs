// Copyright (C) Parity Technologies (UK) Ltd.
// This file is part of Polkadot.

// Polkadot is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Polkadot is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Polkadot.  If not, see <http://www.gnu.org/licenses/>.

//! Block import logic for the approval voting subsystem.
//!
//! There are two major concerns when handling block import notifications.
//!   * Determining all new blocks.
//!   * Handling session changes
//!
//! When receiving a block import notification from the overseer, the
//! approval voting subsystem needs to account for the fact that there
//! may have been blocks missed by the notification. It needs to iterate
//! the ancestry of the block notification back to either the last finalized
//! block or a block that is already accounted for within the DB.
//!
//! We maintain a rolling window of session indices. This starts as empty

use polkadot_node_primitives::{
	approval::{
		self as approval_types,
		v1::{BlockApprovalMeta, RelayVRFStory},
	},
	MAX_FINALITY_LAG,
};
use polkadot_node_subsystem::{
	messages::{
		ApprovalDistributionMessage, ChainApiMessage, ChainSelectionMessage, RuntimeApiMessage,
		RuntimeApiRequest,
	},
	overseer, RuntimeApiError, SubsystemError, SubsystemResult,
};
use polkadot_node_subsystem_util::{determine_new_blocks, runtime::RuntimeInfo};
use polkadot_overseer::SubsystemSender;
use polkadot_primitives::{
	node_features,
	vstaging::{CandidateEvent, CandidateReceiptV2 as CandidateReceipt},
	BlockNumber, CandidateHash, ConsensusLog, CoreIndex, GroupIndex, Hash, Header, SessionIndex,
};
use sc_keystore::LocalKeystore;
use sp_consensus_slots::Slot;

use bitvec::order::Lsb0 as BitOrderLsb0;
use futures::{channel::oneshot, prelude::*};

use std::collections::HashMap;

use super::approval_db::v3;
use crate::{
	backend::{Backend, OverlayedBackend},
	criteria::{AssignmentCriteria, OurAssignment},
	get_extended_session_info, get_session_info,
	persisted_entries::CandidateEntry,
};

use polkadot_node_primitives::approval::time::{slot_number_to_tick, Tick};

use super::{State, LOG_TARGET};

#[derive(Debug)]
struct ImportedBlockInfo {
	included_candidates: Vec<(CandidateHash, CandidateReceipt, CoreIndex, GroupIndex)>,
	session_index: SessionIndex,
	assignments: HashMap<CoreIndex, OurAssignment>,
	n_validators: usize,
	relay_vrf_story: RelayVRFStory,
	slot: Slot,
	force_approve: Option<BlockNumber>,
}

struct ImportedBlockInfoEnv<'a> {
	runtime_info: &'a mut RuntimeInfo,
	assignment_criteria: &'a (dyn AssignmentCriteria + Send + Sync),
	keystore: &'a LocalKeystore,
}

#[derive(Debug, thiserror::Error)]
enum ImportedBlockInfoError {
	// NOTE: The `RuntimeApiError` already prints out which request it was,
	//       so it's not necessary to include that here.
	#[error(transparent)]
	RuntimeError(RuntimeApiError),

	#[error("future cancelled while requesting {0}")]
	FutureCancelled(&'static str, futures::channel::oneshot::Canceled),

	#[error(transparent)]
	ApprovalError(approval_types::v1::ApprovalError),

	#[error("block is already finalized")]
	BlockAlreadyFinalized,

	#[error("session info unavailable")]
	SessionInfoUnavailable,

	#[error("VRF info unavailable")]
	VrfInfoUnavailable,
}

/// Computes information about the imported block. Returns an error if the info couldn't be
/// extracted.
#[overseer::contextbounds(ApprovalVoting, prefix = self::overseer)]
async fn imported_block_info<Sender: SubsystemSender<RuntimeApiMessage>>(
	sender: &mut Sender,
	env: ImportedBlockInfoEnv<'_>,
	block_hash: Hash,
	block_header: &Header,
	last_finalized_height: &Option<BlockNumber>,
) -> Result<ImportedBlockInfo, ImportedBlockInfoError> {
	// Ignore any runtime API errors - that means these blocks are old and finalized.
	// Only unfinalized blocks factor into the approval voting process.

	// fetch candidates
	let included_candidates: Vec<_> = {
		let (c_tx, c_rx) = oneshot::channel();
		sender
			.send_message(RuntimeApiMessage::Request(
				block_hash,
				RuntimeApiRequest::CandidateEvents(c_tx),
			))
			.await;

		let events: Vec<CandidateEvent> = match c_rx.await {
			Ok(Ok(events)) => events,
			Ok(Err(error)) => return Err(ImportedBlockInfoError::RuntimeError(error)),
			Err(error) =>
				return Err(ImportedBlockInfoError::FutureCancelled("CandidateEvents", error)),
		};

		events
			.into_iter()
			.filter_map(|e| match e {
				CandidateEvent::CandidateIncluded(receipt, _, core, group) =>
					Some((receipt.hash(), receipt, core, group)),
				_ => None,
			})
			.collect()
	};

	// fetch session. ignore blocks that are too old, but unless sessions are really
	// short, that shouldn't happen.
	let session_index = {
		let (s_tx, s_rx) = oneshot::channel();
		sender
			.send_message(RuntimeApiMessage::Request(
				block_header.parent_hash,
				RuntimeApiRequest::SessionIndexForChild(s_tx),
			))
			.await;

		let session_index = match s_rx.await {
			Ok(Ok(s)) => s,
			Ok(Err(error)) => return Err(ImportedBlockInfoError::RuntimeError(error)),
			Err(error) =>
				return Err(ImportedBlockInfoError::FutureCancelled("SessionIndexForChild", error)),
		};

		// We can't determine if the block is finalized or not - try processing it
		if last_finalized_height.map_or(false, |finalized| block_header.number < finalized) {
			gum::debug!(
				target: LOG_TARGET,
				session = session_index,
				finalized = ?last_finalized_height,
				"Block {} is either finalized or last finalized height is unknown. Skipping",
				block_hash,
			);

			return Err(ImportedBlockInfoError::BlockAlreadyFinalized)
		}

		session_index
	};

	let babe_epoch = {
		let (s_tx, s_rx) = oneshot::channel();

		// It's not obvious whether to use the hash or the parent hash for this, intuitively. We
		// want to use the block hash itself, and here's why:
		//
		// First off, 'epoch' in BABE means 'session' in other places. 'epoch' is the terminology
		// from the paper, which we fulfill using 'session's, which are a Substrate consensus
		// concept.
		//
		// In BABE, the on-chain and off-chain view of the current epoch can differ at epoch
		// boundaries because epochs change precisely at a slot. When a block triggers a new epoch,
		// the state of its parent will still have the old epoch. Conversely, we have the invariant
		// that every block in BABE has the epoch _it was authored in_ within its post-state. So we
		// use the block, and not its parent.
		//
		// It's worth nothing that Polkadot session changes, at least for the purposes of
		// parachains, would function the same way, except for the fact that they're always delayed
		// by one block. This gives us the opposite invariant for sessions - the parent block's
		// post-state gives us the canonical information about the session index for any of its
		// children, regardless of which slot number they might be produced at.
		sender
			.send_message(RuntimeApiMessage::Request(
				block_hash,
				RuntimeApiRequest::CurrentBabeEpoch(s_tx),
			))
			.await;

		match s_rx.await {
			Ok(Ok(s)) => s,
			Ok(Err(error)) => return Err(ImportedBlockInfoError::RuntimeError(error)),
			Err(error) =>
				return Err(ImportedBlockInfoError::FutureCancelled("CurrentBabeEpoch", error)),
		}
	};

	let extended_session_info =
		get_extended_session_info(env.runtime_info, sender, block_hash, session_index).await;
	let enable_v2_assignments = extended_session_info.map_or(false, |extended_session_info| {
		*extended_session_info
			.node_features
			.get(node_features::FeatureIndex::EnableAssignmentsV2 as usize)
			.as_deref()
			.unwrap_or(&false)
	});

	let session_info = get_session_info(env.runtime_info, sender, block_hash, session_index)
		.await
		.ok_or(ImportedBlockInfoError::SessionInfoUnavailable)?;

	gum::debug!(target: LOG_TARGET, ?enable_v2_assignments, "V2 assignments");
	let (assignments, slot, relay_vrf_story) = {
		let unsafe_vrf = approval_types::v1::babe_unsafe_vrf_info(&block_header);

		match unsafe_vrf {
			Some(unsafe_vrf) => {
				let slot = unsafe_vrf.slot();

				match unsafe_vrf.compute_randomness(
					&babe_epoch.authorities,
					&babe_epoch.randomness,
					babe_epoch.epoch_index,
				) {
					Ok(relay_vrf) => {
						let assignments = env.assignment_criteria.compute_assignments(
							&env.keystore,
							relay_vrf.clone(),
							&crate::criteria::Config::from(session_info),
							included_candidates
								.iter()
								.map(|(c_hash, _, core, group)| (*c_hash, *core, *group))
								.collect(),
							enable_v2_assignments,
						);

						(assignments, slot, relay_vrf)
					},
					Err(error) => return Err(ImportedBlockInfoError::ApprovalError(error)),
				}
			},
			None => {
				gum::debug!(
					target: LOG_TARGET,
					"BABE VRF info unavailable for block {}",
					block_hash,
				);

				return Err(ImportedBlockInfoError::VrfInfoUnavailable)
			},
		}
	};

	gum::trace!(target: LOG_TARGET, n_assignments = assignments.len(), "Produced assignments");

	let force_approve =
		block_header.digest.convert_first(|l| match ConsensusLog::from_digest_item(l) {
			Ok(Some(ConsensusLog::ForceApprove(num))) if num < block_header.number => {
				gum::trace!(
					target: LOG_TARGET,
					?block_hash,
					current_number = block_header.number,
					approved_number = num,
					"Force-approving based on header digest"
				);

				Some(num)
			},
			Ok(Some(_)) => None,
			Ok(None) => None,
			Err(err) => {
				gum::warn!(
					target: LOG_TARGET,
					?err,
					?block_hash,
					"Malformed consensus digest in header",
				);

				None
			},
		});

	Ok(ImportedBlockInfo {
		included_candidates,
		session_index,
		assignments,
		n_validators: session_info.validators.len(),
		relay_vrf_story,
		slot,
		force_approve,
	})
}

/// Information about a block and imported candidates.
pub struct BlockImportedCandidates {
	pub block_hash: Hash,
	pub block_number: BlockNumber,
	pub block_tick: Tick,
	pub imported_candidates: Vec<(CandidateHash, CandidateEntry)>,
}

/// Handle a new notification of a header. This will
///   * determine all blocks to import,
///   * extract candidate information from them
///   * update the rolling session window
///   * compute our assignments
///   * import the block and candidates to the approval DB
///   * and return information about all candidates imported under each block.
///
/// It is the responsibility of the caller to schedule wakeups for each block.
pub(crate) async fn handle_new_head<
	Sender: SubsystemSender<ChainApiMessage>
		+ SubsystemSender<RuntimeApiMessage>
		+ SubsystemSender<ChainSelectionMessage>,
	AVSender: SubsystemSender<ApprovalDistributionMessage>,
	B: Backend,
>(
	sender: &mut Sender,
	approval_voting_sender: &mut AVSender,
	state: &State,
	db: &mut OverlayedBackend<'_, B>,
	session_info_provider: &mut RuntimeInfo,
	head: Hash,
	finalized_number: &Option<BlockNumber>,
) -> SubsystemResult<Vec<BlockImportedCandidates>> {
	const MAX_HEADS_LOOK_BACK: BlockNumber = MAX_FINALITY_LAG;

	let header = {
		let (h_tx, h_rx) = oneshot::channel();
		sender.send_message(ChainApiMessage::BlockHeader(head, h_tx)).await;
		match h_rx.await? {
			Err(e) => {
				gum::debug!(
					target: LOG_TARGET,
					"Chain API subsystem temporarily unreachable {}",
					e,
				);
				// May be a better way of handling errors here.
				return Ok(Vec::new())
			},
			Ok(None) => {
				gum::warn!(target: LOG_TARGET, "Missing header for new head {}", head);
				// May be a better way of handling warnings here.
				return Ok(Vec::new())
			},
			Ok(Some(h)) => h,
		}
	};

	// If we've just started the node and are far behind,
	// import at most `MAX_HEADS_LOOK_BACK` blocks.
	let lower_bound_number = header.number.saturating_sub(MAX_HEADS_LOOK_BACK);
	let lower_bound_number = finalized_number.unwrap_or(lower_bound_number).max(lower_bound_number);

	let new_blocks = determine_new_blocks(
		sender,
		|h| db.load_block_entry(h).map(|e| e.is_some()),
		head,
		&header,
		lower_bound_number,
	)
	.map_err(|e| SubsystemError::with_origin("approval-voting", e))
	.await?;

	if new_blocks.is_empty() {
		return Ok(Vec::new())
	}

	let mut approval_meta: Vec<BlockApprovalMeta> = Vec::with_capacity(new_blocks.len());
	let mut imported_candidates = Vec::with_capacity(new_blocks.len());

	// `determine_new_blocks` gives us a vec in backwards order. we want to move forwards.
	let imported_blocks_and_info = {
		let mut imported_blocks_and_info = Vec::with_capacity(new_blocks.len());
		for (block_hash, block_header) in new_blocks.into_iter().rev() {
			let env = ImportedBlockInfoEnv {
				runtime_info: session_info_provider,
				assignment_criteria: &*state.assignment_criteria,
				keystore: &state.keystore,
			};

			match imported_block_info(sender, env, block_hash, &block_header, finalized_number)
				.await
			{
				Ok(i) => imported_blocks_and_info.push((block_hash, block_header, i)),
				Err(error) => {
					// It's possible that we've lost a race with finality.
					let (tx, rx) = oneshot::channel();
					sender
						.send_message(ChainApiMessage::FinalizedBlockHash(block_header.number, tx))
						.await;

					let lost_to_finality = match rx.await {
						Ok(Ok(Some(h))) if h != block_hash => true,
						_ => false,
					};

					if !lost_to_finality {
						// Such errors are likely spurious, but this prevents us from getting gaps
						// in the approval-db.
						gum::warn!(
							target: LOG_TARGET,
							"Skipping chain: unable to gather info about imported block {:?}: {}",
							(block_hash, block_header.number),
							error,
						);
					}

					return Ok(Vec::new())
				},
			};
		}

		imported_blocks_and_info
	};

	gum::trace!(
		target: LOG_TARGET,
		imported_blocks = imported_blocks_and_info.len(),
		"Inserting imported blocks into database"
	);

	for (block_hash, block_header, imported_block_info) in imported_blocks_and_info {
		let ImportedBlockInfo {
			included_candidates,
			session_index,
			assignments,
			n_validators,
			relay_vrf_story,
			slot,
			force_approve,
		} = imported_block_info;

		let session_info =
			match get_session_info(session_info_provider, sender, head, session_index).await {
				Some(session_info) => session_info,
				None => return Ok(Vec::new()),
			};

		let block_tick = slot_number_to_tick(state.slot_duration_millis, slot);

		let needed_approvals = session_info.needed_approvals;
		let validator_group_lens: Vec<usize> =
			session_info.validator_groups.iter().map(|v| v.len()).collect();
		// insta-approve candidates on low-node testnets:
		// cf. https://github.com/paritytech/polkadot/issues/2411
		let num_candidates = included_candidates.len();
		let approved_bitfield = {
			if needed_approvals == 0 {
				gum::debug!(
					target: LOG_TARGET,
					block_hash = ?block_hash,
					"Insta-approving all candidates",
				);
				bitvec::bitvec![u8, BitOrderLsb0; 1; num_candidates]
			} else {
				let mut result = bitvec::bitvec![u8, BitOrderLsb0; 0; num_candidates];
				for (i, &(_, _, _, backing_group)) in included_candidates.iter().enumerate() {
					let backing_group_size =
						validator_group_lens.get(backing_group.0 as usize).copied().unwrap_or(0);
					let needed_approvals =
						usize::try_from(needed_approvals).expect("usize is at least u32; qed");
					if n_validators.saturating_sub(backing_group_size) < needed_approvals {
						result.set(i, true);
					}
				}
				if result.any() {
					gum::debug!(
						target: LOG_TARGET,
						block_hash = ?block_hash,
						"Insta-approving {}/{} candidates as the number of validators is too low",
						result.count_ones(),
						result.len(),
					);
				}
				result
			}
		};
		// If all bits are already set, then send an approve message.
		if approved_bitfield.count_ones() == approved_bitfield.len() {
			sender.send_message(ChainSelectionMessage::Approved(block_hash)).await;
		}

		let block_entry = v3::BlockEntry {
			block_hash,
			parent_hash: block_header.parent_hash,
			block_number: block_header.number,
			session: session_index,
			slot,
			relay_vrf_story: relay_vrf_story.0,
			candidates: included_candidates
				.iter()
				.map(|(hash, _, core, _)| (*core, *hash))
				.collect(),
			approved_bitfield,
			children: Vec::new(),
			candidates_pending_signature: Default::default(),
			distributed_assignments: Default::default(),
		};

		gum::trace!(
			target: LOG_TARGET,
			?block_hash,
			block_number = block_header.number,
			"Writing BlockEntry",
		);

		let candidate_entries =
			crate::ops::add_block_entry(db, block_entry.into(), n_validators, |candidate_hash| {
				included_candidates.iter().find(|(hash, _, _, _)| candidate_hash == hash).map(
					|(_, receipt, core, backing_group)| {
						super::ops::NewCandidateInfo::new(
							receipt.clone(),
							*backing_group,
							assignments.get(core).map(|a| a.clone().into()),
						)
					},
				)
			})
			.map_err(|e| SubsystemError::with_origin("approval-voting", e))?;

		// force-approve needs to load the current block entry as well as all
		// ancestors. this can only be done after writing the block entry above.
		if let Some(up_to) = force_approve {
			gum::debug!(target: LOG_TARGET, ?block_hash, up_to, "Enacting force-approve");
			let approved_hashes = crate::ops::force_approve(db, block_hash, up_to)
				.map_err(|e| SubsystemError::with_origin("approval-voting", e))?;
			gum::debug!(
				target: LOG_TARGET,
				?block_hash,
				up_to,
				"Force-approving {} blocks",
				approved_hashes.len()
			);

			// Notify chain-selection of all approved hashes.
			for hash in approved_hashes {
				sender.send_message(ChainSelectionMessage::Approved(hash)).await;
			}
		}

		approval_meta.push(BlockApprovalMeta {
			hash: block_hash,
			number: block_header.number,
			parent_hash: block_header.parent_hash,
			candidates: included_candidates
				.iter()
				.map(|(hash, _, core_index, group_index)| (*hash, *core_index, *group_index))
				.collect(),
			slot,
			session: session_index,
			vrf_story: relay_vrf_story,
		});

		imported_candidates.push(BlockImportedCandidates {
			block_hash,
			block_number: block_header.number,
			block_tick,
			imported_candidates: candidate_entries
				.into_iter()
				.map(|(h, e)| (h, e.into()))
				.collect(),
		});
	}

	gum::trace!(
		target: LOG_TARGET,
		head = ?head,
		chain_length = approval_meta.len(),
		"Informing distribution of newly imported chain",
	);

	approval_voting_sender
		.send_unbounded_message(ApprovalDistributionMessage::NewBlocks(approval_meta));
	Ok(imported_candidates)
}

#[cfg(test)]
pub(crate) mod tests {
	use super::*;
	use crate::{
		approval_db::common::{load_block_entry, DbBackend},
		RuntimeInfo, RuntimeInfoConfig, MAX_BLOCKS_WITH_ASSIGNMENT_TIMESTAMPS,
	};
	use approval_types::time::Clock;
	use assert_matches::assert_matches;
	use polkadot_node_primitives::{
		approval::v1::{VrfSignature, VrfTranscript},
		DISPUTE_WINDOW,
	};
	use polkadot_node_subsystem::{
		messages::{AllMessages, ApprovalVotingMessage},
		SubsystemContext,
	};
	use polkadot_node_subsystem_test_helpers::make_subsystem_context;
	use polkadot_node_subsystem_util::database::Database;
	use polkadot_primitives::{
		node_features::FeatureIndex, vstaging::MutateDescriptorV2, ExecutorParams, Id as ParaId,
		IndexedVec, NodeFeatures, SessionInfo, ValidatorId, ValidatorIndex,
	};
	use polkadot_primitives_test_helpers::{dummy_candidate_receipt_v2, dummy_hash};
	use schnellru::{ByLength, LruMap};
	pub(crate) use sp_consensus_babe::{
		digests::{CompatibleDigestItem, PreDigest, SecondaryVRFPreDigest},
		AllowedSlots, BabeEpochConfiguration, Epoch as BabeEpoch,
	};
	use sp_core::{crypto::VrfSecret, testing::TaskExecutor};
	use sp_keyring::sr25519::Keyring as Sr25519Keyring;
	pub(crate) use sp_runtime::{Digest, DigestItem};
	use std::{pin::Pin, sync::Arc};

	use crate::{approval_db::common::Config as DatabaseConfig, criteria, BlockEntry};

	const DATA_COL: u32 = 0;

	const NUM_COLUMNS: u32 = 1;

	const TEST_CONFIG: DatabaseConfig = DatabaseConfig { col_approval_data: DATA_COL };
	#[derive(Default)]
	struct MockClock;

	impl Clock for MockClock {
		fn tick_now(&self) -> Tick {
			42 // chosen by fair dice roll
		}

		fn wait(&self, _tick: Tick) -> Pin<Box<dyn Future<Output = ()> + Send + 'static>> {
			Box::pin(async move { () })
		}
	}

	fn blank_state() -> State {
		State {
			keystore: Arc::new(LocalKeystore::in_memory()),
			slot_duration_millis: 6_000,
			clock: Arc::new(MockClock::default()),
			assignment_criteria: Box::new(MockAssignmentCriteria::default()),
			per_block_assignments_gathering_times: LruMap::new(ByLength::new(
				MAX_BLOCKS_WITH_ASSIGNMENT_TIMESTAMPS,
			)),
			no_show_stats: Default::default(),
		}
	}

	fn single_session_state() -> (State, RuntimeInfo) {
		(
			blank_state(),
			RuntimeInfo::new_with_config(RuntimeInfoConfig {
				keystore: None,
				session_cache_lru_size: DISPUTE_WINDOW.get(),
			}),
		)
	}

	#[derive(Default)]
	struct MockAssignmentCriteria {
		enable_v2: bool,
	}

	impl AssignmentCriteria for MockAssignmentCriteria {
		fn compute_assignments(
			&self,
			_keystore: &LocalKeystore,
			_relay_vrf_story: polkadot_node_primitives::approval::v1::RelayVRFStory,
			_config: &criteria::Config,
			_leaving_cores: Vec<(
				CandidateHash,
				polkadot_primitives::CoreIndex,
				polkadot_primitives::GroupIndex,
			)>,
			enable_assignments_v2: bool,
		) -> HashMap<polkadot_primitives::CoreIndex, criteria::OurAssignment> {
			assert_eq!(enable_assignments_v2, self.enable_v2);
			HashMap::new()
		}

		fn check_assignment_cert(
			&self,
			_claimed_core_bitfield: polkadot_node_primitives::approval::v2::CoreBitfield,
			_validator_index: polkadot_primitives::ValidatorIndex,
			_config: &criteria::Config,
			_relay_vrf_story: polkadot_node_primitives::approval::v1::RelayVRFStory,
			_assignment: &polkadot_node_primitives::approval::v2::AssignmentCertV2,
			_backing_groups: Vec<polkadot_primitives::GroupIndex>,
		) -> Result<polkadot_node_primitives::approval::v1::DelayTranche, criteria::InvalidAssignment>
		{
			Ok(0)
		}
	}

	// used for generating assignments where the validity of the VRF doesn't matter.
	pub(crate) fn garbage_vrf_signature() -> VrfSignature {
		let transcript = VrfTranscript::new(b"test-garbage", &[]);
		Sr25519Keyring::Alice.pair().vrf_sign(&transcript.into())
	}

	fn dummy_session_info(index: SessionIndex) -> SessionInfo {
		SessionInfo {
			validators: Default::default(),
			discovery_keys: Vec::new(),
			assignment_keys: Vec::new(),
			validator_groups: Default::default(),
			n_cores: index as _,
			zeroth_delay_tranche_width: index as _,
			relay_vrf_modulo_samples: index as _,
			n_delay_tranches: index as _,
			no_show_slots: index as _,
			needed_approvals: index as _,
			active_validator_indices: Vec::new(),
			dispute_period: 6,
			random_seed: [0u8; 32],
		}
	}

	#[test]
	fn imported_block_info_is_good() {
		for enable_v2 in [false, true] {
			let pool = TaskExecutor::new();
			let (mut ctx, mut handle) =
				make_subsystem_context::<ApprovalVotingMessage, _>(pool.clone());

			let session = 5;
			let session_info = dummy_session_info(session);

			let slot = Slot::from(10);
			let header = Header {
				digest: {
					let mut d = Digest::default();
					let vrf_signature = garbage_vrf_signature();
					d.push(DigestItem::babe_pre_digest(PreDigest::SecondaryVRF(
						SecondaryVRFPreDigest { authority_index: 0, slot, vrf_signature },
					)));

					d
				},
				extrinsics_root: Default::default(),
				number: 5,
				state_root: Default::default(),
				parent_hash: Default::default(),
			};

			let hash = header.hash();
			let make_candidate = |para_id| {
				let mut r = dummy_candidate_receipt_v2(dummy_hash());
				r.descriptor.set_para_id(para_id);
				r.descriptor.set_relay_parent(hash);
				r
			};
			let candidates = vec![
				(make_candidate(1.into()), CoreIndex(0), GroupIndex(2)),
				(make_candidate(2.into()), CoreIndex(1), GroupIndex(3)),
			];

			let inclusion_events = candidates
				.iter()
				.cloned()
				.map(|(r, c, g)| CandidateEvent::CandidateIncluded(r, Vec::new().into(), c, g))
				.collect::<Vec<_>>();

			let test_fut = {
				let included_candidates = candidates
					.iter()
					.map(|(r, c, g)| (r.hash(), r.clone(), *c, *g))
					.collect::<Vec<_>>();

				let mut runtime_info = RuntimeInfo::new_with_config(RuntimeInfoConfig {
					keystore: None,
					session_cache_lru_size: DISPUTE_WINDOW.get(),
				});

				let header = header.clone();
				Box::pin(async move {
					let env = ImportedBlockInfoEnv {
						runtime_info: &mut runtime_info,
						assignment_criteria: &MockAssignmentCriteria { enable_v2 },
						keystore: &LocalKeystore::in_memory(),
					};

					let info = imported_block_info(ctx.sender(), env, hash, &header, &Some(4))
						.await
						.unwrap();

					assert_eq!(info.included_candidates, included_candidates);
					assert_eq!(info.session_index, session);
					assert!(info.assignments.is_empty());
					assert_eq!(info.n_validators, 0);
					assert_eq!(info.slot, slot);
					assert!(info.force_approve.is_none());
				})
			};

			let aux_fut = Box::pin(async move {
				assert_matches!(
					handle.recv().await,
					AllMessages::RuntimeApi(RuntimeApiMessage::Request(
						h,
						RuntimeApiRequest::CandidateEvents(c_tx),
					)) => {
						assert_eq!(h, hash);
						let _ = c_tx.send(Ok(inclusion_events));
					}
				);

				assert_matches!(
					handle.recv().await,
					AllMessages::RuntimeApi(RuntimeApiMessage::Request(
						h,
						RuntimeApiRequest::SessionIndexForChild(c_tx),
					)) => {
						assert_eq!(h, header.parent_hash);
						let _ = c_tx.send(Ok(session));
					}
				);

				assert_matches!(
					handle.recv().await,
					AllMessages::RuntimeApi(RuntimeApiMessage::Request(
						h,
						RuntimeApiRequest::CurrentBabeEpoch(c_tx),
					)) => {
						assert_eq!(h, hash);
						let _ = c_tx.send(Ok(BabeEpoch {
							epoch_index: session as _,
							start_slot: Slot::from(0),
							duration: 200,
							authorities: vec![(Sr25519Keyring::Alice.public().into(), 1)],
							randomness: [0u8; 32],
							config: BabeEpochConfiguration {
								c: (1, 4),
								allowed_slots: AllowedSlots::PrimarySlots,
							},
						}));
					}
				);

				assert_matches!(
					handle.recv().await,
					AllMessages::RuntimeApi(
						RuntimeApiMessage::Request(
							req_block_hash,
							RuntimeApiRequest::SessionInfo(idx, si_tx),
						)
					) => {
						assert_eq!(session, idx);
						assert_eq!(req_block_hash, hash);
						si_tx.send(Ok(Some(session_info.clone()))).unwrap();
					}
				);

				assert_matches!(
					handle.recv().await,
					AllMessages::RuntimeApi(
						RuntimeApiMessage::Request(
							req_block_hash,
							RuntimeApiRequest::SessionExecutorParams(idx, si_tx),
						)
					) => {
						assert_eq!(session, idx);
						assert_eq!(req_block_hash, hash);
						si_tx.send(Ok(Some(ExecutorParams::default()))).unwrap();
					}
				);

				assert_matches!(
					handle.recv().await,
					AllMessages::RuntimeApi(
						RuntimeApiMessage::Request(_, RuntimeApiRequest::NodeFeatures(_, si_tx), )
					) => {
						si_tx.send(Ok(NodeFeatures::repeat(enable_v2, FeatureIndex::EnableAssignmentsV2 as usize + 1))).unwrap();
					}
				);
			});

			futures::executor::block_on(futures::future::join(test_fut, aux_fut));
		}
	}

	#[test]
	fn imported_block_info_fails_if_no_babe_vrf() {
		let pool = TaskExecutor::new();
		let (mut ctx, mut handle) =
			make_subsystem_context::<ApprovalVotingMessage, _>(pool.clone());

		let session = 5;
		let session_info = dummy_session_info(session);

		let header = Header {
			digest: Digest::default(),
			extrinsics_root: Default::default(),
			number: 5,
			state_root: Default::default(),
			parent_hash: Default::default(),
		};

		let hash = header.hash();
		let make_candidate = |para_id| {
			let mut r = dummy_candidate_receipt_v2(dummy_hash());
			r.descriptor.set_para_id(para_id);
			r.descriptor.set_relay_parent(hash);
			r
		};
		let candidates = vec![
			(make_candidate(1.into()), CoreIndex(0), GroupIndex(2)),
			(make_candidate(2.into()), CoreIndex(1), GroupIndex(3)),
		];

		let inclusion_events = candidates
			.iter()
			.cloned()
			.map(|(r, c, g)| CandidateEvent::CandidateIncluded(r, Vec::new().into(), c, g))
			.collect::<Vec<_>>();

		let test_fut = {
			let mut runtime_info = RuntimeInfo::new_with_config(RuntimeInfoConfig {
				keystore: None,
				session_cache_lru_size: DISPUTE_WINDOW.get(),
			});

			let header = header.clone();
			Box::pin(async move {
				let env = ImportedBlockInfoEnv {
					runtime_info: &mut runtime_info,
					assignment_criteria: &MockAssignmentCriteria::default(),
					keystore: &LocalKeystore::in_memory(),
				};

				let info = imported_block_info(ctx.sender(), env, hash, &header, &Some(4)).await;

				assert_matches!(info, Err(ImportedBlockInfoError::VrfInfoUnavailable));
			})
		};

		let aux_fut = Box::pin(async move {
			assert_matches!(
				handle.recv().await,
				AllMessages::RuntimeApi(RuntimeApiMessage::Request(
					h,
					RuntimeApiRequest::CandidateEvents(c_tx),
				)) => {
					assert_eq!(h, hash);
					let _ = c_tx.send(Ok(inclusion_events));
				}
			);

			assert_matches!(
				handle.recv().await,
				AllMessages::RuntimeApi(RuntimeApiMessage::Request(
					h,
					RuntimeApiRequest::SessionIndexForChild(c_tx),
				)) => {
					assert_eq!(h, header.parent_hash);
					let _ = c_tx.send(Ok(session));
				}
			);

			assert_matches!(
				handle.recv().await,
				AllMessages::RuntimeApi(RuntimeApiMessage::Request(
					h,
					RuntimeApiRequest::CurrentBabeEpoch(c_tx),
				)) => {
					assert_eq!(h, hash);
					let _ = c_tx.send(Ok(BabeEpoch {
						epoch_index: session as _,
						start_slot: Slot::from(0),
						duration: 200,
						authorities: vec![(Sr25519Keyring::Alice.public().into(), 1)],
						randomness: [0u8; 32],
						config: BabeEpochConfiguration {
							c: (1, 4),
							allowed_slots: AllowedSlots::PrimarySlots,
						},
					}));
				}
			);

			assert_matches!(
				handle.recv().await,
				AllMessages::RuntimeApi(
					RuntimeApiMessage::Request(
						req_block_hash,
						RuntimeApiRequest::SessionInfo(idx, si_tx),
					)
				) => {
					assert_eq!(session, idx);
					assert_eq!(req_block_hash, hash);
					si_tx.send(Ok(Some(session_info.clone()))).unwrap();
				}
			);

			assert_matches!(
				handle.recv().await,
				AllMessages::RuntimeApi(
					RuntimeApiMessage::Request(
						req_block_hash,
						RuntimeApiRequest::SessionExecutorParams(idx, si_tx),
					)
				) => {
					assert_eq!(session, idx);
					assert_eq!(req_block_hash, hash);
					si_tx.send(Ok(Some(ExecutorParams::default()))).unwrap();
				}
			);

			assert_matches!(
				handle.recv().await,
				AllMessages::RuntimeApi(
					RuntimeApiMessage::Request(_, RuntimeApiRequest::NodeFeatures(_, si_tx), )
				) => {
					si_tx.send(Ok(NodeFeatures::EMPTY)).unwrap();
				}
			);
		});

		futures::executor::block_on(futures::future::join(test_fut, aux_fut));
	}

	#[test]
	fn imported_block_info_fails_if_ancient_session() {
		let pool = TaskExecutor::new();
		let (mut ctx, mut handle) =
			make_subsystem_context::<ApprovalVotingMessage, _>(pool.clone());

		let session = 5;

		let header = Header {
			digest: Digest::default(),
			extrinsics_root: Default::default(),
			number: 5,
			state_root: Default::default(),
			parent_hash: Default::default(),
		};

		let hash = header.hash();
		let make_candidate = |para_id| {
			let mut r = dummy_candidate_receipt_v2(dummy_hash());
			r.descriptor.set_para_id(para_id);
			r.descriptor.set_relay_parent(hash);
			r
		};
		let candidates = vec![
			(make_candidate(1.into()), CoreIndex(0), GroupIndex(2)),
			(make_candidate(2.into()), CoreIndex(1), GroupIndex(3)),
		];

		let inclusion_events = candidates
			.iter()
			.cloned()
			.map(|(r, c, g)| CandidateEvent::CandidateIncluded(r, Vec::new().into(), c, g))
			.collect::<Vec<_>>();

		let test_fut = {
			let mut runtime_info = RuntimeInfo::new_with_config(RuntimeInfoConfig {
				keystore: None,
				session_cache_lru_size: DISPUTE_WINDOW.get(),
			});

			let header = header.clone();
			Box::pin(async move {
				let env = ImportedBlockInfoEnv {
					runtime_info: &mut runtime_info,
					assignment_criteria: &MockAssignmentCriteria::default(),
					keystore: &LocalKeystore::in_memory(),
				};

				let info = imported_block_info(ctx.sender(), env, hash, &header, &Some(6)).await;

				assert_matches!(info, Err(ImportedBlockInfoError::BlockAlreadyFinalized));
			})
		};

		let aux_fut = Box::pin(async move {
			assert_matches!(
				handle.recv().await,
				AllMessages::RuntimeApi(RuntimeApiMessage::Request(
					h,
					RuntimeApiRequest::CandidateEvents(c_tx),
				)) => {
					assert_eq!(h, hash);
					let _ = c_tx.send(Ok(inclusion_events));
				}
			);

			assert_matches!(
				handle.recv().await,
				AllMessages::RuntimeApi(RuntimeApiMessage::Request(
					h,
					RuntimeApiRequest::SessionIndexForChild(c_tx),
				)) => {
					assert_eq!(h, header.parent_hash);
					let _ = c_tx.send(Ok(session));
				}
			);
		});

		futures::executor::block_on(futures::future::join(test_fut, aux_fut));
	}

	#[test]
	fn imported_block_info_extracts_force_approve() {
		let pool = TaskExecutor::new();
		let (mut ctx, mut handle) =
			make_subsystem_context::<ApprovalVotingMessage, _>(pool.clone());

		let session = 5;
		let session_info = dummy_session_info(session);

		let slot = Slot::from(10);

		let header = Header {
			digest: {
				let mut d = Digest::default();
				let vrf_signature = garbage_vrf_signature();
				d.push(DigestItem::babe_pre_digest(PreDigest::SecondaryVRF(
					SecondaryVRFPreDigest { authority_index: 0, slot, vrf_signature },
				)));

				d.push(ConsensusLog::ForceApprove(3).into());

				d
			},
			extrinsics_root: Default::default(),
			number: 5,
			state_root: Default::default(),
			parent_hash: Default::default(),
		};

		let hash = header.hash();
		let make_candidate = |para_id| {
			let mut r = dummy_candidate_receipt_v2(dummy_hash());
			r.descriptor.set_para_id(para_id);
			r.descriptor.set_relay_parent(hash);
			r
		};
		let candidates = vec![
			(make_candidate(1.into()), CoreIndex(0), GroupIndex(2)),
			(make_candidate(2.into()), CoreIndex(1), GroupIndex(3)),
		];

		let inclusion_events = candidates
			.iter()
			.cloned()
			.map(|(r, c, g)| CandidateEvent::CandidateIncluded(r, Vec::new().into(), c, g))
			.collect::<Vec<_>>();

		let test_fut = {
			let included_candidates = candidates
				.iter()
				.map(|(r, c, g)| (r.hash(), r.clone(), *c, *g))
				.collect::<Vec<_>>();

			let mut runtime_info = RuntimeInfo::new_with_config(RuntimeInfoConfig {
				keystore: None,
				session_cache_lru_size: DISPUTE_WINDOW.get(),
			});

			let header = header.clone();
			Box::pin(async move {
				let env = ImportedBlockInfoEnv {
					runtime_info: &mut runtime_info,
					assignment_criteria: &MockAssignmentCriteria::default(),
					keystore: &LocalKeystore::in_memory(),
				};

				let info =
					imported_block_info(ctx.sender(), env, hash, &header, &Some(4)).await.unwrap();

				assert_eq!(info.included_candidates, included_candidates);
				assert_eq!(info.session_index, session);
				assert!(info.assignments.is_empty());
				assert_eq!(info.n_validators, 0);
				assert_eq!(info.slot, slot);
				assert_eq!(info.force_approve, Some(3));
			})
		};

		let aux_fut = Box::pin(async move {
			assert_matches!(
				handle.recv().await,
				AllMessages::RuntimeApi(RuntimeApiMessage::Request(
					h,
					RuntimeApiRequest::CandidateEvents(c_tx),
				)) => {
					assert_eq!(h, hash);
					let _ = c_tx.send(Ok(inclusion_events));
				}
			);

			assert_matches!(
				handle.recv().await,
				AllMessages::RuntimeApi(RuntimeApiMessage::Request(
					h,
					RuntimeApiRequest::SessionIndexForChild(c_tx),
				)) => {
					assert_eq!(h, header.parent_hash);
					let _ = c_tx.send(Ok(session));
				}
			);

			assert_matches!(
				handle.recv().await,
				AllMessages::RuntimeApi(RuntimeApiMessage::Request(
					h,
					RuntimeApiRequest::CurrentBabeEpoch(c_tx),
				)) => {
					assert_eq!(h, hash);
					let _ = c_tx.send(Ok(BabeEpoch {
						epoch_index: session as _,
						start_slot: Slot::from(0),
						duration: 200,
						authorities: vec![(Sr25519Keyring::Alice.public().into(), 1)],
						randomness: [0u8; 32],
						config: BabeEpochConfiguration {
							c: (1, 4),
							allowed_slots: AllowedSlots::PrimarySlots,
						},
					}));
				}
			);

			assert_matches!(
				handle.recv().await,
				AllMessages::RuntimeApi(
					RuntimeApiMessage::Request(
						req_block_hash,
						RuntimeApiRequest::SessionInfo(idx, si_tx),
					)
				) => {
					assert_eq!(session, idx);
					assert_eq!(req_block_hash, hash);
					si_tx.send(Ok(Some(session_info.clone()))).unwrap();
				}
			);

			assert_matches!(
				handle.recv().await,
				AllMessages::RuntimeApi(
					RuntimeApiMessage::Request(
						req_block_hash,
						RuntimeApiRequest::SessionExecutorParams(idx, si_tx),
					)
				) => {
					assert_eq!(session, idx);
					assert_eq!(req_block_hash, hash);
					si_tx.send(Ok(Some(ExecutorParams::default()))).unwrap();
				}
			);

			assert_matches!(
				handle.recv().await,
				AllMessages::RuntimeApi(
					RuntimeApiMessage::Request(_, RuntimeApiRequest::NodeFeatures(_, si_tx), )
				) => {
					si_tx.send(Ok(NodeFeatures::EMPTY)).unwrap();
				}
			);
		});

		futures::executor::block_on(futures::future::join(test_fut, aux_fut));
	}

	#[test]
	fn insta_approval_works() {
		let db = kvdb_memorydb::create(NUM_COLUMNS);
		let db = polkadot_node_subsystem_util::database::kvdb_impl::DbAdapter::new(db, &[]);
		let db_writer: Arc<dyn Database> = Arc::new(db);
		let mut db = DbBackend::new(db_writer.clone(), TEST_CONFIG);
		let mut overlay_db = OverlayedBackend::new(&db);

		let pool = TaskExecutor::new();
		let (mut ctx, mut handle) =
			make_subsystem_context::<ApprovalVotingMessage, _>(pool.clone());

		let session = 5;
		let irrelevant = 666;
		let session_info =
			SessionInfo {
				validators: IndexedVec::<ValidatorIndex, ValidatorId>::from(
					vec![Sr25519Keyring::Alice.public().into(); 6],
				),
				discovery_keys: Vec::new(),
				assignment_keys: Vec::new(),
				validator_groups: IndexedVec::<GroupIndex, Vec<ValidatorIndex>>::from(vec![
					vec![ValidatorIndex(0); 5],
					vec![ValidatorIndex(0); 2],
				]),
				n_cores: 6,
				needed_approvals: 2,
				zeroth_delay_tranche_width: irrelevant,
				relay_vrf_modulo_samples: irrelevant,
				n_delay_tranches: irrelevant,
				no_show_slots: irrelevant,
				active_validator_indices: Vec::new(),
				dispute_period: 6,
				random_seed: [0u8; 32],
			};

		let slot = Slot::from(10);

		let parent_hash = Hash::repeat_byte(0x01);

		let header = Header {
			digest: {
				let mut d = Digest::default();
				let vrf_signature = garbage_vrf_signature();
				d.push(DigestItem::babe_pre_digest(PreDigest::SecondaryVRF(
					SecondaryVRFPreDigest { authority_index: 0, slot, vrf_signature },
				)));

				d
			},
			extrinsics_root: Default::default(),
			number: 5,
			state_root: Default::default(),
			parent_hash,
		};

		let hash = header.hash();
		let make_candidate = |para_id| {
			let mut r = dummy_candidate_receipt_v2(dummy_hash());
			r.descriptor.set_para_id(para_id);
			r.descriptor.set_relay_parent(hash);
			r
		};
		let candidates = vec![
			(make_candidate(ParaId::from(1)), CoreIndex(0), GroupIndex(0)),
			(make_candidate(ParaId::from(2)), CoreIndex(1), GroupIndex(1)),
		];
		let inclusion_events = candidates
			.iter()
			.cloned()
			.map(|(r, c, g)| CandidateEvent::CandidateIncluded(r, Vec::new().into(), c, g))
			.collect::<Vec<_>>();

		let (state, mut session_info_provider) = single_session_state();
		overlay_db.write_block_entry(
			v3::BlockEntry {
				block_hash: parent_hash,
				parent_hash: Default::default(),
				block_number: 4,
				session,
				slot,
				relay_vrf_story: Default::default(),
				candidates: Vec::new(),
				approved_bitfield: Default::default(),
				children: Vec::new(),
				candidates_pending_signature: Default::default(),
				distributed_assignments: Default::default(),
			}
			.into(),
		);

		let write_ops = overlay_db.into_write_ops();
		db.write(write_ops).unwrap();

		let test_fut = {
			Box::pin(async move {
				let mut overlay_db = OverlayedBackend::new(&db);

				let mut approval_voting_sender = ctx.sender().clone();
				let result = handle_new_head(
					ctx.sender(),
					&mut approval_voting_sender,
					&state,
					&mut overlay_db,
					&mut session_info_provider,
					hash,
					&Some(1),
				)
				.await
				.unwrap();

				let write_ops = overlay_db.into_write_ops();
				db.write(write_ops).unwrap();

				assert_eq!(result.len(), 1);
				let candidates = &result[0].imported_candidates;
				assert_eq!(candidates.len(), 2);
				assert_eq!(candidates[0].1.approvals().len(), 6);
				assert_eq!(candidates[1].1.approvals().len(), 6);
				// the first candidate should be insta-approved
				// the second should not
				let entry: BlockEntry = load_block_entry(db_writer.as_ref(), &TEST_CONFIG, &hash)
					.unwrap()
					.unwrap()
					.into();
				assert!(entry.is_candidate_approved(&candidates[0].0));
				assert!(!entry.is_candidate_approved(&candidates[1].0));
			})
		};

		let aux_fut = Box::pin(async move {
			assert_matches!(
				handle.recv().await,
				AllMessages::ChainApi(ChainApiMessage::BlockHeader(
					h,
					tx,
				)) => {
					assert_eq!(h, hash);
					let _ = tx.send(Ok(Some(header.clone())));
				}
			);

			// determine_new_blocks exits early as the parent_hash is in the DB

			assert_matches!(
				handle.recv().await,
				AllMessages::RuntimeApi(RuntimeApiMessage::Request(
					h,
					RuntimeApiRequest::CandidateEvents(c_tx),
				)) => {
					assert_eq!(h, hash.clone());
					let _ = c_tx.send(Ok(inclusion_events));
				}
			);

			assert_matches!(
				handle.recv().await,
				AllMessages::RuntimeApi(RuntimeApiMessage::Request(
					h,
					RuntimeApiRequest::SessionIndexForChild(c_tx),
				)) => {
					assert_eq!(h, parent_hash.clone());
					let _ = c_tx.send(Ok(session));
				}
			);

			assert_matches!(
				handle.recv().await,
				AllMessages::RuntimeApi(RuntimeApiMessage::Request(
					h,
					RuntimeApiRequest::CurrentBabeEpoch(c_tx),
				)) => {
					assert_eq!(h, hash);
					let _ = c_tx.send(Ok(BabeEpoch {
						epoch_index: session as _,
						start_slot: Slot::from(0),
						duration: 200,
						authorities: vec![(Sr25519Keyring::Alice.public().into(), 1)],
						randomness: [0u8; 32],
						config: BabeEpochConfiguration {
							c: (1, 4),
							allowed_slots: AllowedSlots::PrimarySlots,
						},
					}));
				}
			);

			assert_matches!(
				handle.recv().await,
				AllMessages::RuntimeApi(
					RuntimeApiMessage::Request(
						req_block_hash,
						RuntimeApiRequest::SessionInfo(idx, si_tx),
					)
				) => {
					assert_eq!(session, idx);
					assert_eq!(req_block_hash, hash);
					si_tx.send(Ok(Some(session_info.clone()))).unwrap();
				}
			);

			assert_matches!(
				handle.recv().await,
				AllMessages::RuntimeApi(
					RuntimeApiMessage::Request(
						req_block_hash,
						RuntimeApiRequest::SessionExecutorParams(idx, si_tx),
					)
				) => {
					assert_eq!(session, idx);
					assert_eq!(req_block_hash, hash);
					si_tx.send(Ok(Some(ExecutorParams::default()))).unwrap();
				}
			);

			assert_matches!(
				handle.recv().await,
				AllMessages::RuntimeApi(
					RuntimeApiMessage::Request(_, RuntimeApiRequest::NodeFeatures(_, si_tx), )
				) => {
					si_tx.send(Ok(NodeFeatures::EMPTY)).unwrap();
				}
			);

			assert_matches!(
				handle.recv().await,
				AllMessages::ApprovalDistribution(ApprovalDistributionMessage::NewBlocks(
					approval_meta
				)) => {
					assert_eq!(approval_meta.len(), 1);
				}
			);
		});

		futures::executor::block_on(futures::future::join(test_fut, aux_fut));
	}
}
