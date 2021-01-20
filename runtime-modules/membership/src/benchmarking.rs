#![cfg(feature = "runtime-benchmarks")]
use super::*;
use crate::{
    BuyMembershipParameters, MemberIdByHandleHash, Membership, MembershipById, MembershipObject,
    Trait,
};
use balances::Module as Balances;
use common::working_group::MembershipWorkingGroupHelper;
use core::convert::TryInto;
use frame_benchmarking::{account, benchmarks};
use frame_support::storage::StorageMap;
use frame_support::traits::Currency;
use frame_system::Module as System;
use frame_system::{EventRecord, RawOrigin};
use sp_arithmetic::traits::{One, Zero};
use sp_runtime::traits::Bounded;
use sp_std::prelude::*;

/// Balance alias for `balances` module.
pub type BalanceOf<T> = <T as balances::Trait>::Balance;

const SEED: u32 = 0;
const MAX_BYTES: u32 = 16384;

fn get_byte(num: u32, byte_number: u8) -> u8 {
    ((num & (0xff << (8 * byte_number))) >> 8 * byte_number) as u8
}

fn assert_last_event<T: Trait>(generic_event: <T as Trait>::Event) {
    let events = System::<T>::events();
    let system_event: <T as frame_system::Trait>::Event = generic_event.into();
    // compare to the last event record
    let EventRecord { event, .. } = &events[events.len() - 1];
    assert_eq!(event, &system_event);
}

fn member_funded_account<T: Trait + balances::Trait>(
    name: &'static str,
    id: u32,
) -> (T::AccountId, T::MemberId) {
    let account_id = account::<T::AccountId>(name, id, SEED);

    let handle = handle_from_id::<T>(id);

    let _ = Balances::<T>::make_free_balance_be(&account_id, BalanceOf::<T>::max_value());

    let params = BuyMembershipParameters {
        root_account: account_id.clone(),
        controller_account: account_id.clone(),
        name: None,
        handle: Some(handle),
        avatar_uri: None,
        about: None,
        referrer_id: None,
    };

    Module::<T>::buy_membership(RawOrigin::Signed(account_id.clone()).into(), params).unwrap();

    let _ = Balances::<T>::make_free_balance_be(&account_id, BalanceOf::<T>::max_value());

    let member_id = T::MemberId::from(id.try_into().unwrap());
    Module::<T>::add_staking_account_candidate(
        RawOrigin::Signed(account_id.clone()).into(),
        member_id.clone(),
    )
    .unwrap();
    Module::<T>::confirm_staking_account(
        RawOrigin::Signed(account_id.clone()).into(),
        member_id.clone(),
        account_id.clone(),
    )
    .unwrap();

    (account_id, member_id)
}

// Method to generate a distintic valid handle
// for a membership. For each index.
fn handle_from_id<T: Trait>(id: u32) -> Vec<u8> {
    let mut handle = vec![];

    for j in 0..4 {
        handle.push(get_byte(id, j));
    }

    while handle.len() < (id as usize) {
        handle.push(0u8);
    }

    handle
}

benchmarks! {
    where_clause { where T: balances::Trait, T: Trait }
    _{  }

    buy_membership_without_referrer{

        let i in 0 .. MAX_BYTES;

        let member_id = 0;

        let account_id = account::<T::AccountId>("member", member_id, SEED);

        let handle = handle_from_id::<T>(i);

        let member_id = T::MemberId::from(member_id.try_into().unwrap());

        let free_balance = BalanceOf::<T>::max_value();

        let _ = Balances::<T>::make_free_balance_be(&account_id, free_balance);

        let fee = Module::<T>::membership_price();

        let params = BuyMembershipParameters {
            root_account: account_id.clone(),
            controller_account: account_id.clone(),
            name: None,
            handle: Some(handle.clone()),
            avatar_uri: None,
            about: None,
            referrer_id: None,
        };

    }: buy_membership(RawOrigin::Signed(account_id.clone()), params)
    verify {

        assert_eq!(Module::<T>::members_created(), member_id + T::MemberId::one());

        assert_eq!(Balances::<T>::free_balance(&account_id.clone()), free_balance - fee);

        let handle_hash = T::Hashing::hash(&handle).as_ref().to_vec();

        let membership: Membership<T> = MembershipObject {
            handle_hash: handle_hash.clone(),
            root_account: account_id.clone(),
            controller_account: account_id.clone(),
            verified: false,
            // Save the updated profile.
            invites: 5,
        };

        assert_eq!(MemberIdByHandleHash::<T>::get(&handle_hash), member_id);

        assert_eq!(MembershipById::<T>::get(member_id), membership);

        assert_last_event::<T>(RawEvent::MemberRegistered(member_id).into());
    }

    buy_membership_with_referrer{

        let i in 0 .. MAX_BYTES;

        let member_id = 0;

        let account_id = account::<T::AccountId>("member", member_id, SEED);

        let handle = handle_from_id::<T>(i);

        let _ = Balances::<T>::make_free_balance_be(&account_id, BalanceOf::<T>::max_value());

        let fee = Module::<T>::membership_price();

        let mut params = BuyMembershipParameters {
            root_account: account_id.clone(),
            controller_account: account_id.clone(),
            name: None,
            handle: Some(handle.clone()),
            avatar_uri: None,
            about: None,
            referrer_id: None,
        };

        Module::<T>::buy_membership(RawOrigin::Signed(account_id.clone()).into(), params.clone());

        let referral_cut: BalanceOf<T> = 1.into();

        Module::<T>::set_referral_cut(RawOrigin::Root.into(), referral_cut);

        let member_id = T::MemberId::from(member_id.try_into().unwrap());

        params.referrer_id = Some(member_id);
        let second_handle = handle_from_id::<T>(i + 1);

        params.handle = Some(second_handle.clone());

        let free_balance = Balances::<T>::free_balance(&account_id);

    }: buy_membership(RawOrigin::Signed(account_id.clone()), params)
    verify {

        assert_eq!(Module::<T>::members_created(), member_id + T::MemberId::one() + T::MemberId::one());

        // Same account id gets reward for being referral.
        assert_eq!(Balances::<T>::free_balance(&account_id.clone()), free_balance - fee + referral_cut);

        let second_handle_hash = T::Hashing::hash(&second_handle).as_ref().to_vec();

        let membership: Membership<T> = MembershipObject {
            handle_hash: second_handle_hash.clone(),
            root_account: account_id.clone(),
            controller_account: account_id.clone(),
            verified: false,
            // Save the updated profile.
            invites: 5,
        };

        let second_member_id = member_id + T::MemberId::one();

        assert_eq!(MemberIdByHandleHash::<T>::get(second_handle_hash), second_member_id);

        assert_eq!(MembershipById::<T>::get(second_member_id), membership);

        assert_last_event::<T>(RawEvent::MemberRegistered(second_member_id).into());
    }

    update_profile{

        let i in 0 .. MAX_BYTES;

        let member_id = 0;

        let account_id = account::<T::AccountId>("member", member_id, SEED);

        let handle = handle_from_id::<T>(i);

        let _ = Balances::<T>::make_free_balance_be(&account_id, BalanceOf::<T>::max_value());

        let member_id = T::MemberId::from(member_id.try_into().unwrap());

        let params = BuyMembershipParameters {
            root_account: account_id.clone(),
            controller_account: account_id.clone(),
            name: None,
            handle: Some(handle.clone()),
            avatar_uri: None,
            about: None,
            referrer_id: None,
        };

        Module::<T>::buy_membership(RawOrigin::Signed(account_id.clone()).into(), params.clone());

        let handle_updated = handle_from_id::<T>(i + 1);

    }: _ (RawOrigin::Signed(account_id.clone()), member_id, None, Some(handle_updated.clone()), None, None)
    verify {

        let handle_hash = T::Hashing::hash(&handle_updated).as_ref().to_vec();

        assert!(!MemberIdByHandleHash::<T>::contains_key(handle));

        assert_eq!(MemberIdByHandleHash::<T>::get(handle_updated), member_id);

        assert_last_event::<T>(RawEvent::MemberProfileUpdated(member_id).into());
    }

    update_accounts_none{

        let member_id = 0;

        let (account_id, member_id) = member_funded_account::<T>("member", member_id);

    }: update_accounts(RawOrigin::Signed(account_id.clone()), member_id, None, None)

    update_accounts_root{

        let member_id = 0;

        let new_root_account_id = account::<T::AccountId>("root", member_id, SEED);

        let handle = handle_from_id::<T>(member_id);

        let (account_id, member_id) = member_funded_account::<T>("member", member_id);

    }: update_accounts(RawOrigin::Signed(account_id.clone()), member_id, Some(new_root_account_id.clone()), None)

    verify {
        let handle_hash = T::Hashing::hash(&handle).as_ref().to_vec();

        let membership: Membership<T> = MembershipObject {
            handle_hash: handle_hash.clone(),
            root_account: new_root_account_id.clone(),
            controller_account: account_id.clone(),
            verified: false,
            // Save the updated profile.
            invites: 5,
        };

        assert_eq!(MembershipById::<T>::get(member_id), membership);

        assert_last_event::<T>(RawEvent::MemberAccountsUpdated(member_id).into());
    }

    update_accounts_controller{

        let member_id = 0;

        let new_controller_account_id = account::<T::AccountId>("controller", member_id, SEED);

        let handle = handle_from_id::<T>(member_id);

        let (account_id, member_id) = member_funded_account::<T>("member", member_id);

    }: update_accounts(RawOrigin::Signed(account_id.clone()), member_id, None, Some(new_controller_account_id.clone()))

    verify {
        let handle_hash = T::Hashing::hash(&handle).as_ref().to_vec();

        let membership: Membership<T> = MembershipObject {
            handle_hash: handle_hash.clone(),
            root_account: account_id.clone(),
            controller_account: new_controller_account_id.clone(),
            verified: false,
            // Save the updated profile.
            invites: 5,
        };

        assert_eq!(MembershipById::<T>::get(member_id), membership);

        assert_last_event::<T>(RawEvent::MemberAccountsUpdated(member_id).into());
    }

    update_accounts_both{

        let member_id = 0;

        let new_controller_account_id = account::<T::AccountId>("controller", member_id, SEED);

        let new_root_account_id = account::<T::AccountId>("root", member_id, SEED);

        let handle = handle_from_id::<T>(member_id);

        let (account_id, member_id) = member_funded_account::<T>("member", member_id);

    }: update_accounts(RawOrigin::Signed(account_id.clone()), member_id, Some(new_root_account_id.clone()), Some(new_controller_account_id.clone()))

    verify {
        let handle_hash = T::Hashing::hash(&handle).as_ref().to_vec();

        let membership: Membership<T> = MembershipObject {
            handle_hash: handle_hash.clone(),
            root_account: new_root_account_id.clone(),
            controller_account: new_controller_account_id.clone(),
            verified: false,
            // Save the updated profile.
            invites: 5,
        };

        assert_eq!(MembershipById::<T>::get(member_id), membership);

        assert_last_event::<T>(RawEvent::MemberAccountsUpdated(member_id).into());
    }

    set_referral_cut {
        let member_id = 0;

        let referral_cut: BalanceOf<T> = 1.into();

    }: _(RawOrigin::Root, referral_cut)

    verify {

        assert_eq!(Module::<T>::referral_cut(), referral_cut);

        assert_last_event::<T>(RawEvent::ReferralCutUpdated(referral_cut).into());
    }

    transfer_invites{

        let first_member_id = 0;

        let second_member_id = 1;

        let first_handle = handle_from_id::<T>(first_member_id);
        let (first_account_id, first_member_id) = member_funded_account::<T>("first_member", first_member_id);

        let second_handle = handle_from_id::<T>(second_member_id);
        let (second_account_id, second_member_id) = member_funded_account::<T>("second_member", second_member_id);

        let number_of_invites = 5;

    }: _(RawOrigin::Signed(first_account_id.clone()), first_member_id, second_member_id, number_of_invites)

    verify {
        let first_handle_hash = T::Hashing::hash(&first_handle).as_ref().to_vec();

        let second_handle_hash = T::Hashing::hash(&second_handle).as_ref().to_vec();

        let first_membership: Membership<T> = MembershipObject {
            handle_hash: first_handle_hash,
            root_account: first_account_id.clone(),
            controller_account: first_account_id.clone(),
            verified: false,
            // Save the updated profile.
            invites: 0,
        };

        let second_membership: Membership<T> = MembershipObject {
            handle_hash: second_handle_hash,
            root_account: second_account_id.clone(),
            controller_account: second_account_id.clone(),
            verified: false,
            // Save the updated profile.
            invites: 10,
        };

        assert_eq!(MembershipById::<T>::get(first_member_id), first_membership);

        assert_eq!(MembershipById::<T>::get(second_member_id), second_membership);

        assert_last_event::<T>(RawEvent::InvitesTransferred(first_member_id, second_member_id, number_of_invites).into());
    }

    set_membership_price {
        let membership_price: BalanceOf<T> = 1000.into();

    }: _(RawOrigin::Root, membership_price)
    verify {
        assert_eq!(Module::<T>::membership_price(), membership_price);

        assert_last_event::<T>(RawEvent::MembershipPriceUpdated(membership_price).into());
    }

    set_leader_invitation_quota {
        // Set leader member id

        let member_id = 0;

        let (account_id, member_id) = member_funded_account::<T>("member", member_id);

        // Set leader member id
        T::WorkingGroup::insert_a_lead(0, &account_id, member_id);

        let leader_member_id = T::WorkingGroup::get_leader_member_id();

        let invitation_quota = 100;

    }: _(RawOrigin::Root, invitation_quota)
    verify {

        assert_eq!(MembershipById::<T>::get(leader_member_id.unwrap()).invites, invitation_quota);

        assert_last_event::<T>(RawEvent::LeaderInvitationQuotaUpdated(invitation_quota).into());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::*;
    use frame_support::assert_ok;

    #[test]
    fn buy_membership_with_referrer() {
        build_test_externalities().execute_with(|| {
            assert_ok!(test_benchmark_buy_membership_with_referrer::<Test>());
        });
    }

    #[test]
    fn buy_membership_without_referrer() {
        build_test_externalities().execute_with(|| {
            assert_ok!(test_benchmark_buy_membership_without_referrer::<Test>());
        });
    }

    #[test]
    fn update_profile() {
        build_test_externalities().execute_with(|| {
            assert_ok!(test_benchmark_update_profile::<Test>());
        });
    }

    #[test]
    fn update_accounts_none() {
        build_test_externalities().execute_with(|| {
            assert_ok!(test_benchmark_update_accounts_none::<Test>());
        });
    }

    #[test]
    fn update_accounts_root() {
        build_test_externalities().execute_with(|| {
            assert_ok!(test_benchmark_update_accounts_root::<Test>());
        });
    }

    #[test]
    fn update_accounts_controller() {
        build_test_externalities().execute_with(|| {
            assert_ok!(test_benchmark_update_accounts_controller::<Test>());
        });
    }

    #[test]
    fn update_accounts_both() {
        build_test_externalities().execute_with(|| {
            assert_ok!(test_benchmark_update_accounts_both::<Test>());
        });
    }

    #[test]
    fn set_referral_cut() {
        build_test_externalities().execute_with(|| {
            assert_ok!(test_benchmark_set_referral_cut::<Test>());
        });
    }

    #[test]
    fn transfer_invites() {
        build_test_externalities().execute_with(|| {
            assert_ok!(test_benchmark_transfer_invites::<Test>());
        });
    }

    #[test]
    fn set_membership_price() {
        build_test_externalities().execute_with(|| {
            assert_ok!(test_benchmark_set_membership_price::<Test>());
        });
    }

    #[test]
    fn set_leader_invitation_quota() {
        build_test_externalities().execute_with(|| {
            assert_ok!(test_benchmark_set_leader_invitation_quota::<Test>());
        });
    }
}
