// Copyright (C) Parity Technologies (UK) Ltd.
// This file is part of Cumulus.
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// 	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Generating test data for bridges with remote parachains.

use super::{
	from_grandpa_chain::make_complex_bridged_grandpa_header_proof, prepare_inbound_xcm,
	XcmAsPlainPayload,
};

use bp_messages::{
	source_chain::FromBridgedChainMessagesDeliveryProof,
	target_chain::FromBridgedChainMessagesProof, ChainWithMessages, LaneState,
	UnrewardedRelayersState, Weight,
};
use bp_parachains::{RelayBlockHash, RelayBlockNumber};
use bp_runtime::{
	AccountIdOf, BlockNumberOf, Chain, HeaderOf, Parachain, UnverifiedStorageProofParams,
};
use bp_test_utils::prepare_parachain_heads_proof;
use codec::Encode;
use pallet_bridge_grandpa::BridgedHeader;
use sp_runtime::traits::Header as HeaderT;
use xcm::latest::prelude::*;

use crate::test_cases::helpers::InboundRelayerId;
use bp_header_chain::{justification::GrandpaJustification, ChainWithGrandpa};
use bp_messages::{DeliveredMessages, InboundLaneData, MessageNonce, UnrewardedRelayer};
use bp_polkadot_core::parachains::{ParaHash, ParaHead, ParaHeadsProof, ParaId};
use pallet_bridge_messages::{
	messages_generation::{
		encode_all_messages, encode_lane_data, prepare_message_delivery_storage_proof,
		prepare_messages_storage_proof,
	},
	BridgedChainOf, LaneIdOf,
};
use sp_runtime::SaturatedConversion;

/// Prepare a batch call with relay finality proof, parachain head proof and message proof.
pub fn make_complex_relayer_delivery_batch<Runtime, GPI, PPI, MPI>(
	relay_chain_header: BridgedHeader<Runtime, GPI>,
	grandpa_justification: GrandpaJustification<BridgedHeader<Runtime, GPI>>,
	parachain_heads: Vec<(ParaId, ParaHash)>,
	para_heads_proof: ParaHeadsProof,
	message_proof: FromBridgedChainMessagesProof<ParaHash, LaneIdOf<Runtime, MPI>>,
	relayer_id_at_bridged_chain: InboundRelayerId<Runtime, MPI>,
) -> pallet_utility::Call<Runtime>
where
	Runtime: pallet_bridge_grandpa::Config<GPI>
		+ pallet_bridge_parachains::Config<PPI>
		+ pallet_bridge_messages::Config<MPI, InboundPayload = XcmAsPlainPayload>
		+ pallet_utility::Config,
	GPI: 'static,
	PPI: 'static,
	MPI: 'static,
	ParaHash: From<
		<<Runtime as pallet_bridge_grandpa::Config<GPI>>::BridgedChain as bp_runtime::Chain>::Hash,
	>,
	<<Runtime as pallet_bridge_grandpa::Config<GPI>>::BridgedChain as bp_runtime::Chain>::Hash:
		From<ParaHash>,
	BridgedChainOf<Runtime, MPI>: Chain<Hash = ParaHash> + Parachain,
	<Runtime as pallet_utility::Config>::RuntimeCall: From<pallet_bridge_grandpa::Call<Runtime, GPI>>
		+ From<pallet_bridge_parachains::Call<Runtime, PPI>>
		+ From<pallet_bridge_messages::Call<Runtime, MPI>>,
{
	let relay_chain_header_hash = relay_chain_header.hash();
	let relay_chain_header_number = *relay_chain_header.number();
	let submit_grandpa = pallet_bridge_grandpa::Call::<Runtime, GPI>::submit_finality_proof {
		finality_target: Box::new(relay_chain_header),
		justification: grandpa_justification,
	};
	let submit_para_head = pallet_bridge_parachains::Call::<Runtime, PPI>::submit_parachain_heads {
		at_relay_block: (
			relay_chain_header_number.saturated_into(),
			relay_chain_header_hash.into(),
		),
		parachains: parachain_heads,
		parachain_heads_proof: para_heads_proof,
	};
	let submit_message = pallet_bridge_messages::Call::<Runtime, MPI>::receive_messages_proof {
		relayer_id_at_bridged_chain: relayer_id_at_bridged_chain.into(),
		proof: Box::new(message_proof),
		messages_count: 1,
		dispatch_weight: Weight::from_parts(1000000000, 0),
	};
	pallet_utility::Call::<Runtime>::batch_all {
		calls: vec![submit_grandpa.into(), submit_para_head.into(), submit_message.into()],
	}
}

/// Prepare a batch call with relay finality proof, parachain head proof and message delivery
/// proof.
pub fn make_complex_relayer_confirmation_batch<Runtime, GPI, PPI, MPI>(
	relay_chain_header: BridgedHeader<Runtime, GPI>,
	grandpa_justification: GrandpaJustification<BridgedHeader<Runtime, GPI>>,
	parachain_heads: Vec<(ParaId, ParaHash)>,
	para_heads_proof: ParaHeadsProof,
	message_delivery_proof: FromBridgedChainMessagesDeliveryProof<ParaHash, LaneIdOf<Runtime, MPI>>,
	relayers_state: UnrewardedRelayersState,
) -> pallet_utility::Call<Runtime>
where
	Runtime: pallet_bridge_grandpa::Config<GPI>
		+ pallet_bridge_parachains::Config<PPI>
		+ pallet_bridge_messages::Config<MPI, OutboundPayload = XcmAsPlainPayload>
		+ pallet_utility::Config,
	GPI: 'static,
	PPI: 'static,
	MPI: 'static,
	<Runtime as pallet_bridge_grandpa::Config<GPI>>::BridgedChain:
		bp_runtime::Chain<Hash = RelayBlockHash, BlockNumber = RelayBlockNumber> + ChainWithGrandpa,
	BridgedChainOf<Runtime, MPI>: Chain<Hash = ParaHash> + Parachain,
	<Runtime as pallet_utility::Config>::RuntimeCall: From<pallet_bridge_grandpa::Call<Runtime, GPI>>
		+ From<pallet_bridge_parachains::Call<Runtime, PPI>>
		+ From<pallet_bridge_messages::Call<Runtime, MPI>>,
{
	let relay_chain_header_hash = relay_chain_header.hash();
	let relay_chain_header_number = *relay_chain_header.number();
	let submit_grandpa = pallet_bridge_grandpa::Call::<Runtime, GPI>::submit_finality_proof {
		finality_target: Box::new(relay_chain_header),
		justification: grandpa_justification,
	};
	let submit_para_head = pallet_bridge_parachains::Call::<Runtime, PPI>::submit_parachain_heads {
		at_relay_block: (
			relay_chain_header_number.saturated_into(),
			relay_chain_header_hash.into(),
		),
		parachains: parachain_heads,
		parachain_heads_proof: para_heads_proof,
	};
	let submit_message_delivery_proof =
		pallet_bridge_messages::Call::<Runtime, MPI>::receive_messages_delivery_proof {
			proof: message_delivery_proof,
			relayers_state,
		};
	pallet_utility::Call::<Runtime>::batch_all {
		calls: vec![
			submit_grandpa.into(),
			submit_para_head.into(),
			submit_message_delivery_proof.into(),
		],
	}
}

/// Prepare a call with message proof.
pub fn make_standalone_relayer_delivery_call<Runtime, MPI>(
	message_proof: FromBridgedChainMessagesProof<ParaHash, LaneIdOf<Runtime, MPI>>,
	relayer_id_at_bridged_chain: InboundRelayerId<Runtime, MPI>,
) -> Runtime::RuntimeCall
where
	Runtime: pallet_bridge_messages::Config<MPI, InboundPayload = XcmAsPlainPayload>,
	MPI: 'static,
	Runtime::RuntimeCall: From<pallet_bridge_messages::Call<Runtime, MPI>>,
	BridgedChainOf<Runtime, MPI>: Chain<Hash = ParaHash> + Parachain,
{
	pallet_bridge_messages::Call::<Runtime, MPI>::receive_messages_proof {
		relayer_id_at_bridged_chain: relayer_id_at_bridged_chain.into(),
		proof: Box::new(message_proof),
		messages_count: 1,
		dispatch_weight: Weight::from_parts(1000000000, 0),
	}
	.into()
}

/// Prepare a call with message delivery proof.
pub fn make_standalone_relayer_confirmation_call<Runtime, MPI>(
	message_delivery_proof: FromBridgedChainMessagesDeliveryProof<ParaHash, LaneIdOf<Runtime, MPI>>,
	relayers_state: UnrewardedRelayersState,
) -> Runtime::RuntimeCall
where
	Runtime: pallet_bridge_messages::Config<MPI, OutboundPayload = XcmAsPlainPayload>,
	MPI: 'static,
	Runtime::RuntimeCall: From<pallet_bridge_messages::Call<Runtime, MPI>>,
	BridgedChainOf<Runtime, MPI>: Chain<Hash = ParaHash> + Parachain,
{
	pallet_bridge_messages::Call::<Runtime, MPI>::receive_messages_delivery_proof {
		proof: message_delivery_proof,
		relayers_state,
	}
	.into()
}

/// Prepare storage proofs of messages, stored at the source chain.
pub fn make_complex_relayer_delivery_proofs<
	BridgedRelayChain,
	BridgedParachain,
	ThisChainWithMessages,
	LaneId,
>(
	lane_id: LaneId,
	xcm_message: Xcm<()>,
	message_nonce: MessageNonce,
	message_destination: Junctions,
	para_header_number: u32,
	relay_header_number: u32,
	bridged_para_id: u32,
	is_minimal_call: bool,
) -> (
	HeaderOf<BridgedRelayChain>,
	GrandpaJustification<HeaderOf<BridgedRelayChain>>,
	ParaHead,
	Vec<(ParaId, ParaHash)>,
	ParaHeadsProof,
	FromBridgedChainMessagesProof<ParaHash, LaneId>,
)
where
	BridgedRelayChain:
		bp_runtime::Chain<Hash = RelayBlockHash, BlockNumber = RelayBlockNumber> + ChainWithGrandpa,
	BridgedParachain: bp_runtime::Chain<Hash = ParaHash> + Parachain,
	ThisChainWithMessages: ChainWithMessages,
	LaneId: Copy + Encode,
{
	// prepare message
	let message_payload = prepare_inbound_xcm(xcm_message, message_destination);
	// prepare para storage proof containing message
	let (para_state_root, para_storage_proof) =
		prepare_messages_storage_proof::<BridgedParachain, ThisChainWithMessages, LaneId>(
			lane_id,
			message_nonce..=message_nonce,
			None,
			UnverifiedStorageProofParams::from_db_size(message_payload.len() as u32),
			|_| message_payload.clone(),
			encode_all_messages,
			encode_lane_data,
			false,
			false,
		);

	let (relay_chain_header, justification, bridged_para_head, parachain_heads, para_heads_proof) =
		make_complex_bridged_parachain_heads_proof::<BridgedRelayChain, BridgedParachain>(
			para_state_root,
			para_header_number,
			relay_header_number,
			bridged_para_id,
			is_minimal_call,
		);

	let message_proof = FromBridgedChainMessagesProof {
		bridged_header_hash: bridged_para_head.hash(),
		storage_proof: para_storage_proof,
		lane: lane_id,
		nonces_start: message_nonce,
		nonces_end: message_nonce,
	};

	(
		relay_chain_header,
		justification,
		bridged_para_head,
		parachain_heads,
		para_heads_proof,
		message_proof,
	)
}

/// Prepare storage proofs of message confirmations, stored at the target parachain.
pub fn make_complex_relayer_confirmation_proofs<
	BridgedRelayChain,
	BridgedParachain,
	ThisChainWithMessages,
	LaneId,
>(
	lane_id: LaneId,
	para_header_number: u32,
	relay_header_number: u32,
	bridged_para_id: u32,
	relayer_id_at_this_chain: AccountIdOf<ThisChainWithMessages>,
	relayers_state: UnrewardedRelayersState,
) -> (
	HeaderOf<BridgedRelayChain>,
	GrandpaJustification<HeaderOf<BridgedRelayChain>>,
	ParaHead,
	Vec<(ParaId, ParaHash)>,
	ParaHeadsProof,
	FromBridgedChainMessagesDeliveryProof<ParaHash, LaneId>,
)
where
	BridgedRelayChain:
		bp_runtime::Chain<Hash = RelayBlockHash, BlockNumber = RelayBlockNumber> + ChainWithGrandpa,
	BridgedParachain: bp_runtime::Chain<Hash = ParaHash> + Parachain,
	ThisChainWithMessages: ChainWithMessages,
	LaneId: Copy + Encode,
{
	// prepare para storage proof containing message delivery proof
	let (para_state_root, para_storage_proof) =
		prepare_message_delivery_storage_proof::<BridgedParachain, ThisChainWithMessages, LaneId>(
			lane_id,
			InboundLaneData {
				state: LaneState::Opened,
				relayers: vec![
					UnrewardedRelayer {
						relayer: relayer_id_at_this_chain.into(),
						messages: DeliveredMessages::new(1)
					};
					relayers_state.unrewarded_relayer_entries as usize
				]
				.into(),
				last_confirmed_nonce: 1,
			},
			UnverifiedStorageProofParams::default(),
		);

	let (relay_chain_header, justification, bridged_para_head, parachain_heads, para_heads_proof) =
		make_complex_bridged_parachain_heads_proof::<BridgedRelayChain, BridgedParachain>(
			para_state_root,
			para_header_number,
			relay_header_number,
			bridged_para_id,
			false,
		);

	let message_delivery_proof = FromBridgedChainMessagesDeliveryProof {
		bridged_header_hash: bridged_para_head.hash(),
		storage_proof: para_storage_proof,
		lane: lane_id,
	};

	(
		relay_chain_header,
		justification,
		bridged_para_head,
		parachain_heads,
		para_heads_proof,
		message_delivery_proof,
	)
}

/// Make bridged parachain header with given state root and relay header that is finalizing it.
pub fn make_complex_bridged_parachain_heads_proof<BridgedRelayChain, BridgedParachain>(
	para_state_root: ParaHash,
	para_header_number: u32,
	relay_header_number: BlockNumberOf<BridgedRelayChain>,
	bridged_para_id: u32,
	is_minimal_call: bool,
) -> (
	HeaderOf<BridgedRelayChain>,
	GrandpaJustification<HeaderOf<BridgedRelayChain>>,
	ParaHead,
	Vec<(ParaId, ParaHash)>,
	ParaHeadsProof,
)
where
	BridgedRelayChain:
		bp_runtime::Chain<Hash = RelayBlockHash, BlockNumber = RelayBlockNumber> + ChainWithGrandpa,
	BridgedParachain: bp_runtime::Chain<Hash = ParaHash> + Parachain,
{
	let bridged_para_head = ParaHead(
		bp_test_utils::test_header_with_root::<HeaderOf<BridgedParachain>>(
			para_header_number.into(),
			para_state_root,
		)
		.encode(),
	);
	let (relay_state_root, para_heads_proof, parachain_heads) =
		prepare_parachain_heads_proof::<HeaderOf<BridgedParachain>>(vec![(
			bridged_para_id,
			bridged_para_head.clone(),
		)]);
	assert_eq!(bridged_para_head.hash(), parachain_heads[0].1);

	let (relay_chain_header, justification) =
		make_complex_bridged_grandpa_header_proof::<BridgedRelayChain>(
			relay_state_root,
			relay_header_number,
			is_minimal_call,
		);

	(relay_chain_header, justification, bridged_para_head, parachain_heads, para_heads_proof)
}
