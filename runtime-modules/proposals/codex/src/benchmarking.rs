#![cfg(feature = "runtime-benchmarks")]
use super::*;
use crate::Module as ProposalsCodex;
use balances::Module as Balances;
use frame_benchmarking::{account, benchmarks};
use frame_system::EventRecord;
use frame_system::Module as System;
use frame_system::RawOrigin;
use membership::Module as Membership;
use proposals_discussion::Module as ProposalsDiscussion;
use sp_arithmetic::traits::Bounded;
use sp_std::cmp::min;
use sp_std::convert::TryInto;
use sp_std::prelude::*;

const SEED: u32 = 0;

fn get_byte(num: u32, byte_number: u8) -> u8 {
    ((num & (0xff << (8 * byte_number))) >> 8 * byte_number) as u8
}

// Method to generate a distintic valid handle
// for a membership. For each index.
fn handle_from_id<T: membership::Trait>(id: u32) -> Vec<u8> {
    let min_handle_length = Membership::<T>::min_handle_length();

    let mut handle = vec![];

    for i in 0..min(Membership::<T>::max_handle_length().try_into().unwrap(), 4) {
        handle.push(get_byte(id, i));
    }

    while handle.len() < (min_handle_length as usize) {
        handle.push(0u8);
    }

    handle
}

/*
fn assert_last_event<T: Trait>(generic_event: <T as Trait>::Event) {
    let events = System::<T>::events();
    let system_event: <T as frame_system::Trait>::Event = generic_event.into();
    // compare to the last event record
    let EventRecord { event, .. } = &events[events.len() - 1];
    assert_eq!(event, &system_event);
}
*/

fn member_funded_account<T: Trait>(name: &'static str, id: u32) -> (T::AccountId, T::MemberId) {
    let account_id = account::<T::AccountId>(name, id, SEED);
    let handle = handle_from_id::<T>(id);

    let authority_account = account::<T::AccountId>(name, 0, SEED);

    Membership::<T>::set_screening_authority(RawOrigin::Root.into(), authority_account.clone())
        .unwrap();

    Membership::<T>::add_screened_member(
        RawOrigin::Signed(authority_account.clone()).into(),
        account_id.clone(),
        Some(handle),
        None,
        None,
    )
    .unwrap();

    let _ = Balances::<T>::make_free_balance_be(&account_id, T::Balance::max_value());

    (account_id, T::MemberId::from(id.try_into().unwrap()))
}

const MAX_BYTES: u32 = 16384;

benchmarks! {
    _ { }

    create_text_proposal {
        let i in 1 .. T::TitleMaxLength::get();
        let j in 1 .. T::DescriptionMaxLength::get();
        let k in 1 .. T::TextProposalMaxLength::get();

        let (account_id, member_id) = member_funded_account::<T>("caller", 0);

        let exact_block_number = T::BlockNumber::max_value();
    }: _(
        RawOrigin::Signed(account_id.clone()),
        member_id,
        vec![0u8; i.try_into().unwrap()],
        vec![0u8; j.try_into().unwrap()],
        Some(account_id.clone()),
        vec![0u8; k.try_into().unwrap()],
        Some(exact_block_number)
    )
    verify {
        let thread_id = T::ThreadId::from(1);
        assert!(proposals_discussion::ThreadById::<T>::contains_key(thread_id))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::{initial_test_ext, Test};
    use frame_support::assert_ok;

    #[test]
    fn create_text_proposal() {
        initial_test_ext().execute_with(|| {
            assert_ok!(test_benchmark_create_text_proposal::<Test>());
        });
    }
}
