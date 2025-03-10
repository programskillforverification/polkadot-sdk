// Copyright (C) Parity Technologies (UK) Ltd.
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

//! Autogenerated weights for `pallet_bridge_messages`
//!
//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 32.0.0
//! DATE: 2025-02-21, STEPS: `50`, REPEAT: `20`, LOW RANGE: `[]`, HIGH RANGE: `[]`
//! WORST CASE MAP SIZE: `1000000`
//! HOSTNAME: `814af52b0d43`, CPU: `Intel(R) Xeon(R) CPU @ 2.60GHz`
//! WASM-EXECUTION: `Compiled`, CHAIN: `None`, DB CACHE: 1024

// Executed Command:
// frame-omni-bencher
// v1
// benchmark
// pallet
// --extrinsic=*
// --runtime=target/production/wbuild/bridge-hub-rococo-runtime/bridge_hub_rococo_runtime.wasm
// --pallet=pallet_bridge_messages
// --header=/__w/polkadot-sdk/polkadot-sdk/cumulus/file_header.txt
// --output=./cumulus/parachains/runtimes/bridge-hubs/bridge-hub-rococo/src/weights
// --wasm-execution=compiled
// --steps=50
// --repeat=20
// --heap-pages=4096
// --no-storage-info
// --no-min-squares
// --no-median-slopes

#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(unused_parens)]
#![allow(unused_imports)]
#![allow(missing_docs)]

use frame_support::{traits::Get, weights::Weight};
use core::marker::PhantomData;

/// Weight functions for `pallet_bridge_messages`.
pub struct WeightInfo<T>(PhantomData<T>);
impl<T: frame_system::Config> pallet_bridge_messages::WeightInfo for WeightInfo<T> {
	/// Storage: `BridgePolkadotBulletinMessages::PalletOperatingMode` (r:1 w:0)
	/// Proof: `BridgePolkadotBulletinMessages::PalletOperatingMode` (`max_values`: Some(1), `max_size`: Some(2), added: 497, mode: `MaxEncodedLen`)
	/// Storage: `BridgePolkadotBulletinGrandpa::ImportedHeaders` (r:1 w:0)
	/// Proof: `BridgePolkadotBulletinGrandpa::ImportedHeaders` (`max_values`: Some(1024), `max_size`: Some(68), added: 1553, mode: `MaxEncodedLen`)
	/// Storage: `BridgePolkadotBulletinMessages::InboundLanes` (r:1 w:1)
	/// Proof: `BridgePolkadotBulletinMessages::InboundLanes` (`max_values`: None, `max_size`: Some(49180), added: 51655, mode: `MaxEncodedLen`)
	/// Storage: `XcmOverPolkadotBulletin::LaneToBridge` (r:1 w:0)
	/// Proof: `XcmOverPolkadotBulletin::LaneToBridge` (`max_values`: None, `max_size`: Some(36), added: 2511, mode: `MaxEncodedLen`)
	/// Storage: `XcmOverPolkadotBulletin::Bridges` (r:1 w:0)
	/// Proof: `XcmOverPolkadotBulletin::Bridges` (`max_values`: None, `max_size`: Some(1889), added: 4364, mode: `MaxEncodedLen`)
	/// Storage: `ParachainInfo::ParachainId` (r:1 w:0)
	/// Proof: `ParachainInfo::ParachainId` (`max_values`: Some(1), `max_size`: Some(4), added: 499, mode: `MaxEncodedLen`)
	fn receive_single_message_proof() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `581`
		//  Estimated: `52645`
		// Minimum execution time: 48_956_000 picoseconds.
		Weight::from_parts(50_706_000, 0)
			.saturating_add(Weight::from_parts(0, 52645))
			.saturating_add(T::DbWeight::get().reads(6))
			.saturating_add(T::DbWeight::get().writes(1))
	}
	/// Storage: `BridgePolkadotBulletinMessages::PalletOperatingMode` (r:1 w:0)
	/// Proof: `BridgePolkadotBulletinMessages::PalletOperatingMode` (`max_values`: Some(1), `max_size`: Some(2), added: 497, mode: `MaxEncodedLen`)
	/// Storage: `BridgePolkadotBulletinGrandpa::ImportedHeaders` (r:1 w:0)
	/// Proof: `BridgePolkadotBulletinGrandpa::ImportedHeaders` (`max_values`: Some(1024), `max_size`: Some(68), added: 1553, mode: `MaxEncodedLen`)
	/// Storage: `BridgePolkadotBulletinMessages::InboundLanes` (r:1 w:1)
	/// Proof: `BridgePolkadotBulletinMessages::InboundLanes` (`max_values`: None, `max_size`: Some(49180), added: 51655, mode: `MaxEncodedLen`)
	/// Storage: `XcmOverPolkadotBulletin::LaneToBridge` (r:1 w:0)
	/// Proof: `XcmOverPolkadotBulletin::LaneToBridge` (`max_values`: None, `max_size`: Some(36), added: 2511, mode: `MaxEncodedLen`)
	/// Storage: `XcmOverPolkadotBulletin::Bridges` (r:1 w:0)
	/// Proof: `XcmOverPolkadotBulletin::Bridges` (`max_values`: None, `max_size`: Some(1889), added: 4364, mode: `MaxEncodedLen`)
	/// Storage: `ParachainInfo::ParachainId` (r:1 w:0)
	/// Proof: `ParachainInfo::ParachainId` (`max_values`: Some(1), `max_size`: Some(4), added: 499, mode: `MaxEncodedLen`)
	/// The range of component `n` is `[1, 4076]`.
	/// The range of component `n` is `[1, 4076]`.
	fn receive_n_messages_proof(n: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `581`
		//  Estimated: `52645`
		// Minimum execution time: 49_150_000 picoseconds.
		Weight::from_parts(50_060_000, 0)
			.saturating_add(Weight::from_parts(0, 52645))
			// Standard Error: 10_047
			.saturating_add(Weight::from_parts(10_087_182, 0).saturating_mul(n.into()))
			.saturating_add(T::DbWeight::get().reads(6))
			.saturating_add(T::DbWeight::get().writes(1))
	}
	/// Storage: `BridgePolkadotBulletinMessages::PalletOperatingMode` (r:1 w:0)
	/// Proof: `BridgePolkadotBulletinMessages::PalletOperatingMode` (`max_values`: Some(1), `max_size`: Some(2), added: 497, mode: `MaxEncodedLen`)
	/// Storage: `BridgePolkadotBulletinGrandpa::ImportedHeaders` (r:1 w:0)
	/// Proof: `BridgePolkadotBulletinGrandpa::ImportedHeaders` (`max_values`: Some(1024), `max_size`: Some(68), added: 1553, mode: `MaxEncodedLen`)
	/// Storage: `BridgePolkadotBulletinMessages::InboundLanes` (r:1 w:1)
	/// Proof: `BridgePolkadotBulletinMessages::InboundLanes` (`max_values`: None, `max_size`: Some(49180), added: 51655, mode: `MaxEncodedLen`)
	/// Storage: `XcmOverPolkadotBulletin::LaneToBridge` (r:1 w:0)
	/// Proof: `XcmOverPolkadotBulletin::LaneToBridge` (`max_values`: None, `max_size`: Some(36), added: 2511, mode: `MaxEncodedLen`)
	/// Storage: `XcmOverPolkadotBulletin::Bridges` (r:1 w:0)
	/// Proof: `XcmOverPolkadotBulletin::Bridges` (`max_values`: None, `max_size`: Some(1889), added: 4364, mode: `MaxEncodedLen`)
	/// Storage: `ParachainInfo::ParachainId` (r:1 w:0)
	/// Proof: `ParachainInfo::ParachainId` (`max_values`: Some(1), `max_size`: Some(4), added: 499, mode: `MaxEncodedLen`)
	fn receive_single_message_proof_with_outbound_lane_state() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `581`
		//  Estimated: `52645`
		// Minimum execution time: 54_027_000 picoseconds.
		Weight::from_parts(55_948_000, 0)
			.saturating_add(Weight::from_parts(0, 52645))
			.saturating_add(T::DbWeight::get().reads(6))
			.saturating_add(T::DbWeight::get().writes(1))
	}
	/// Storage: `BridgePolkadotBulletinMessages::PalletOperatingMode` (r:1 w:0)
	/// Proof: `BridgePolkadotBulletinMessages::PalletOperatingMode` (`max_values`: Some(1), `max_size`: Some(2), added: 497, mode: `MaxEncodedLen`)
	/// Storage: `BridgePolkadotBulletinGrandpa::ImportedHeaders` (r:1 w:0)
	/// Proof: `BridgePolkadotBulletinGrandpa::ImportedHeaders` (`max_values`: Some(1024), `max_size`: Some(68), added: 1553, mode: `MaxEncodedLen`)
	/// Storage: `BridgePolkadotBulletinMessages::InboundLanes` (r:1 w:1)
	/// Proof: `BridgePolkadotBulletinMessages::InboundLanes` (`max_values`: None, `max_size`: Some(49180), added: 51655, mode: `MaxEncodedLen`)
	/// Storage: `XcmOverPolkadotBulletin::LaneToBridge` (r:1 w:0)
	/// Proof: `XcmOverPolkadotBulletin::LaneToBridge` (`max_values`: None, `max_size`: Some(36), added: 2511, mode: `MaxEncodedLen`)
	/// Storage: `XcmOverPolkadotBulletin::Bridges` (r:1 w:0)
	/// Proof: `XcmOverPolkadotBulletin::Bridges` (`max_values`: None, `max_size`: Some(1889), added: 4364, mode: `MaxEncodedLen`)
	/// Storage: `ParachainInfo::ParachainId` (r:1 w:0)
	/// Proof: `ParachainInfo::ParachainId` (`max_values`: Some(1), `max_size`: Some(4), added: 499, mode: `MaxEncodedLen`)
	/// The range of component `n` is `[1, 16384]`.
	/// The range of component `n` is `[1, 16384]`.
	fn receive_single_n_bytes_message_proof(n: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `581`
		//  Estimated: `52645`
		// Minimum execution time: 47_288_000 picoseconds.
		Weight::from_parts(50_465_963, 0)
			.saturating_add(Weight::from_parts(0, 52645))
			// Standard Error: 10
			.saturating_add(Weight::from_parts(1_880, 0).saturating_mul(n.into()))
			.saturating_add(T::DbWeight::get().reads(6))
			.saturating_add(T::DbWeight::get().writes(1))
	}
	/// Storage: `BridgePolkadotBulletinMessages::PalletOperatingMode` (r:1 w:0)
	/// Proof: `BridgePolkadotBulletinMessages::PalletOperatingMode` (`max_values`: Some(1), `max_size`: Some(2), added: 497, mode: `MaxEncodedLen`)
	/// Storage: `BridgePolkadotBulletinGrandpa::ImportedHeaders` (r:1 w:0)
	/// Proof: `BridgePolkadotBulletinGrandpa::ImportedHeaders` (`max_values`: Some(1024), `max_size`: Some(68), added: 1553, mode: `MaxEncodedLen`)
	/// Storage: `BridgePolkadotBulletinMessages::OutboundLanes` (r:1 w:1)
	/// Proof: `BridgePolkadotBulletinMessages::OutboundLanes` (`max_values`: None, `max_size`: Some(45), added: 2520, mode: `MaxEncodedLen`)
	/// Storage: `XcmOverPolkadotBulletin::LaneToBridge` (r:1 w:0)
	/// Proof: `XcmOverPolkadotBulletin::LaneToBridge` (`max_values`: None, `max_size`: Some(36), added: 2511, mode: `MaxEncodedLen`)
	/// Storage: `XcmOverPolkadotBulletin::Bridges` (r:1 w:0)
	/// Proof: `XcmOverPolkadotBulletin::Bridges` (`max_values`: None, `max_size`: Some(1889), added: 4364, mode: `MaxEncodedLen`)
	/// Storage: `BridgePolkadotBulletinMessages::OutboundMessages` (r:0 w:1)
	/// Proof: `BridgePolkadotBulletinMessages::OutboundMessages` (`max_values`: None, `max_size`: Some(65568), added: 68043, mode: `MaxEncodedLen`)
	fn receive_delivery_proof_for_single_message() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `548`
		//  Estimated: `5354`
		// Minimum execution time: 41_029_000 picoseconds.
		Weight::from_parts(42_595_000, 0)
			.saturating_add(Weight::from_parts(0, 5354))
			.saturating_add(T::DbWeight::get().reads(5))
			.saturating_add(T::DbWeight::get().writes(2))
	}
	/// Storage: `BridgePolkadotBulletinMessages::PalletOperatingMode` (r:1 w:0)
	/// Proof: `BridgePolkadotBulletinMessages::PalletOperatingMode` (`max_values`: Some(1), `max_size`: Some(2), added: 497, mode: `MaxEncodedLen`)
	/// Storage: `BridgePolkadotBulletinGrandpa::ImportedHeaders` (r:1 w:0)
	/// Proof: `BridgePolkadotBulletinGrandpa::ImportedHeaders` (`max_values`: Some(1024), `max_size`: Some(68), added: 1553, mode: `MaxEncodedLen`)
	/// Storage: `BridgePolkadotBulletinMessages::OutboundLanes` (r:1 w:1)
	/// Proof: `BridgePolkadotBulletinMessages::OutboundLanes` (`max_values`: None, `max_size`: Some(45), added: 2520, mode: `MaxEncodedLen`)
	/// Storage: `XcmOverPolkadotBulletin::LaneToBridge` (r:1 w:0)
	/// Proof: `XcmOverPolkadotBulletin::LaneToBridge` (`max_values`: None, `max_size`: Some(36), added: 2511, mode: `MaxEncodedLen`)
	/// Storage: `XcmOverPolkadotBulletin::Bridges` (r:1 w:0)
	/// Proof: `XcmOverPolkadotBulletin::Bridges` (`max_values`: None, `max_size`: Some(1889), added: 4364, mode: `MaxEncodedLen`)
	/// Storage: `BridgePolkadotBulletinMessages::OutboundMessages` (r:0 w:2)
	/// Proof: `BridgePolkadotBulletinMessages::OutboundMessages` (`max_values`: None, `max_size`: Some(65568), added: 68043, mode: `MaxEncodedLen`)
	fn receive_delivery_proof_for_two_messages_by_single_relayer() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `548`
		//  Estimated: `5354`
		// Minimum execution time: 42_745_000 picoseconds.
		Weight::from_parts(44_562_000, 0)
			.saturating_add(Weight::from_parts(0, 5354))
			.saturating_add(T::DbWeight::get().reads(5))
			.saturating_add(T::DbWeight::get().writes(3))
	}
	/// Storage: `BridgePolkadotBulletinMessages::PalletOperatingMode` (r:1 w:0)
	/// Proof: `BridgePolkadotBulletinMessages::PalletOperatingMode` (`max_values`: Some(1), `max_size`: Some(2), added: 497, mode: `MaxEncodedLen`)
	/// Storage: `BridgePolkadotBulletinGrandpa::ImportedHeaders` (r:1 w:0)
	/// Proof: `BridgePolkadotBulletinGrandpa::ImportedHeaders` (`max_values`: Some(1024), `max_size`: Some(68), added: 1553, mode: `MaxEncodedLen`)
	/// Storage: `BridgePolkadotBulletinMessages::OutboundLanes` (r:1 w:1)
	/// Proof: `BridgePolkadotBulletinMessages::OutboundLanes` (`max_values`: None, `max_size`: Some(45), added: 2520, mode: `MaxEncodedLen`)
	/// Storage: `XcmOverPolkadotBulletin::LaneToBridge` (r:1 w:0)
	/// Proof: `XcmOverPolkadotBulletin::LaneToBridge` (`max_values`: None, `max_size`: Some(36), added: 2511, mode: `MaxEncodedLen`)
	/// Storage: `XcmOverPolkadotBulletin::Bridges` (r:1 w:0)
	/// Proof: `XcmOverPolkadotBulletin::Bridges` (`max_values`: None, `max_size`: Some(1889), added: 4364, mode: `MaxEncodedLen`)
	/// Storage: `BridgePolkadotBulletinMessages::OutboundMessages` (r:0 w:2)
	/// Proof: `BridgePolkadotBulletinMessages::OutboundMessages` (`max_values`: None, `max_size`: Some(65568), added: 68043, mode: `MaxEncodedLen`)
	fn receive_delivery_proof_for_two_messages_by_two_relayers() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `548`
		//  Estimated: `5354`
		// Minimum execution time: 42_717_000 picoseconds.
		Weight::from_parts(44_144_000, 0)
			.saturating_add(Weight::from_parts(0, 5354))
			.saturating_add(T::DbWeight::get().reads(5))
			.saturating_add(T::DbWeight::get().writes(3))
	}
	/// Storage: `BridgePolkadotBulletinMessages::PalletOperatingMode` (r:1 w:0)
	/// Proof: `BridgePolkadotBulletinMessages::PalletOperatingMode` (`max_values`: Some(1), `max_size`: Some(2), added: 497, mode: `MaxEncodedLen`)
	/// Storage: `BridgePolkadotBulletinGrandpa::ImportedHeaders` (r:1 w:0)
	/// Proof: `BridgePolkadotBulletinGrandpa::ImportedHeaders` (`max_values`: Some(1024), `max_size`: Some(68), added: 1553, mode: `MaxEncodedLen`)
	/// Storage: `BridgePolkadotBulletinMessages::InboundLanes` (r:1 w:1)
	/// Proof: `BridgePolkadotBulletinMessages::InboundLanes` (`max_values`: None, `max_size`: Some(49180), added: 51655, mode: `MaxEncodedLen`)
	/// Storage: `XcmOverPolkadotBulletin::LaneToBridge` (r:1 w:0)
	/// Proof: `XcmOverPolkadotBulletin::LaneToBridge` (`max_values`: None, `max_size`: Some(36), added: 2511, mode: `MaxEncodedLen`)
	/// Storage: `XcmOverPolkadotBulletin::Bridges` (r:1 w:0)
	/// Proof: `XcmOverPolkadotBulletin::Bridges` (`max_values`: None, `max_size`: Some(1889), added: 4364, mode: `MaxEncodedLen`)
	/// Storage: `ParachainInfo::ParachainId` (r:1 w:0)
	/// Proof: `ParachainInfo::ParachainId` (`max_values`: Some(1), `max_size`: Some(4), added: 499, mode: `MaxEncodedLen`)
	/// Storage: `XcmpQueue::DeliveryFeeFactor` (r:1 w:0)
	/// Proof: `XcmpQueue::DeliveryFeeFactor` (`max_values`: None, `max_size`: Some(28), added: 2503, mode: `MaxEncodedLen`)
	/// Storage: `PolkadotXcm::SupportedVersion` (r:1 w:0)
	/// Proof: `PolkadotXcm::SupportedVersion` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `ParachainSystem::RelevantMessagingState` (r:1 w:0)
	/// Proof: `ParachainSystem::RelevantMessagingState` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	/// Storage: `XcmpQueue::OutboundXcmpStatus` (r:1 w:1)
	/// Proof: `XcmpQueue::OutboundXcmpStatus` (`max_values`: Some(1), `max_size`: Some(1282), added: 1777, mode: `MaxEncodedLen`)
	/// Storage: `XcmpQueue::OutboundXcmpMessages` (r:0 w:1)
	/// Proof: `XcmpQueue::OutboundXcmpMessages` (`max_values`: None, `max_size`: Some(105506), added: 107981, mode: `MaxEncodedLen`)
	/// The range of component `n` is `[1, 16384]`.
	/// The range of component `n` is `[1, 16384]`.
	fn receive_single_n_bytes_message_proof_with_dispatch(n: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `708`
		//  Estimated: `52645`
		// Minimum execution time: 78_507_000 picoseconds.
		Weight::from_parts(84_359_182, 0)
			.saturating_add(Weight::from_parts(0, 52645))
			// Standard Error: 23
			.saturating_add(Weight::from_parts(6_812, 0).saturating_mul(n.into()))
			.saturating_add(T::DbWeight::get().reads(10))
			.saturating_add(T::DbWeight::get().writes(3))
	}
}
