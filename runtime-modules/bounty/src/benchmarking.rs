#![cfg(feature = "runtime-benchmarks")]

use frame_benchmarking::benchmarks;
use frame_support::storage::StorageMap;
use frame_system::Module as System;
use frame_system::{EventRecord, RawOrigin};
use sp_arithmetic::traits::One;
use sp_std::boxed::Box;
use sp_std::vec;
use sp_std::vec::Vec;

use crate::{Bounties, Bounty, BountyCreationParameters, Call, Event, Module, Trait};

fn assert_last_event<T: Trait>(generic_event: <T as Trait>::Event) {
    let events = System::<T>::events();
    let system_event: <T as frame_system::Trait>::Event = generic_event.into();
    // compare to the last event record
    let EventRecord { event, .. } = &events[events.len() - 1];
    assert_eq!(event, &system_event);
}

const MAX_BYTES: u32 = 50000;

benchmarks! {
    _{ }

    create_bounty{
        let i in 1 .. MAX_BYTES;
        let metadata = vec![0u8].repeat(i as usize);

        let params = BountyCreationParameters::<T>{
            work_period: One::one(),
            judging_period: One::one(),
            ..Default::default()
        };

    }: _ (RawOrigin::Root, params.clone(), metadata)
    verify {
        let bounty = Bounty::<T>{
            creation_params: params,
        };

        let bounty_id: T::BountyId = 1u32.into();

        assert_eq!(Module::<T>::bounties(bounty_id), bounty);
        assert_last_event::<T>(Event::<T>::BountyCreated(bounty_id).into());
    }

    cancel_bounty{
        let params = BountyCreationParameters::<T>{
            work_period: One::one(),
            judging_period: One::one(),
            ..Default::default()
        };

        Module::<T>::create_bounty(RawOrigin::Root.into(), params, Vec::new()).unwrap();

        let bounty_id: T::BountyId = Module::<T>::bounty_count().into();

    }: _ (RawOrigin::Root, None, bounty_id)
    verify {
        assert!(!<Bounties<T>>::contains_key(&bounty_id));
        assert_last_event::<T>(Event::<T>::BountyCanceled(bounty_id).into());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::mocks::{build_test_externalities, Test};
    use frame_support::assert_ok;

    #[test]
    fn create_bounty() {
        build_test_externalities().execute_with(|| {
            assert_ok!(test_benchmark_create_bounty::<Test>());
        });
    }

    #[test]
    fn cancel_bounty() {
        build_test_externalities().execute_with(|| {
            assert_ok!(test_benchmark_cancel_bounty::<Test>());
        });
    }
}
