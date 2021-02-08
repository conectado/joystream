//! # Blog Module
//!
//!
//! The Blog module provides functionality for handling blogs
//!
//! - [`timestamp::Trait`](./trait.Trait.html)
//! - [`Call`](./enum.Call.html)
//! - [`Module`](./struct.Module.html)
//!
//! ## Overview
//!
//! The blog module provides functions for:
//!
//! - Creation and editing of posts, associated with given blog
//! - Posts locking/unlocking
//! - Creation and editing of replies, associated with given post
//! - Reactions for both posts and replies
//!
//! ### Terminology
//!
//! - **Lock:** A forbiddance of mutation of any associated information related to a given post.
//!
//! - **Reaction:** A user can react to a post in N different ways, where N is an integer parameter configured through runtime.
//! For each way, the reader can simply react and unreact to a given post. Think of reactions as being things like, unlike,
//! laugh, etc. The semantics of each reaction is not present in the runtime.
//!
//! ## Interface
//! The posts creation/edition/locking/unlocking are done through proposals
//! To reply/react to posts you need to be a member
//!
//! ## Supported extrinsics
//!
//! - [create_post](./struct.Module.html#method.create_post)
//! - [lock_post](./struct.Module.html#method.lock_post)
//! - [unlock_post](./struct.Module.html#method.unlock_post)
//! - [edit_post](./struct.Module.html#method.edit_post)
//! - [create_reply](./struct.Module.html#method.create_reply)
//! - [edit_reply](./struct.Module.html#method.create_reply)
//! - [react](./struct.Module.html#method.create_reply)

#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Codec, Decode, Encode};
use common::origin::MemberOriginValidator;
use errors::Error;
pub use frame_support::dispatch::{DispatchError, DispatchResult};
use frame_support::weights::Weight;
use frame_support::{
    decl_event, decl_module, decl_storage, ensure, traits::Get, Parameter, StorageDoubleMap,
};
use sp_arithmetic::traits::{BaseArithmetic, One};
use sp_runtime::traits::{Hash, MaybeSerialize, Member};
use sp_runtime::SaturatedConversion;
use sp_std::prelude::*;

mod benchmarking;
mod errors;
mod mock;
mod tests;

// Type for maximum number of posts/replies
type MaxNumber = u64;

/// Type for post IDs
pub type PostId = u64;

/// Type, representing reactions number
pub type ReactionsNumber = u64;

/// Number of reactions, presented in runtime
pub const REACTIONS_MAX_NUMBER: ReactionsNumber = 5;

/// Blogger participant ID alias for the member of the system.
pub type ParticipantId<T> = common::MemberId<T>;

/// blog WeightInfo.
/// Note: This was auto generated through the benchmark CLI using the `--weight-trait` flag
pub trait WeightInfo {
    fn create_post(t: u32, b: u32) -> Weight;
    fn lock_post() -> Weight;
    fn unlock_post() -> Weight;
    fn edit_post(t: u32, b: u32) -> Weight;
    fn create_reply_to_post(t: u32) -> Weight;
    fn create_reply_to_reply(t: u32) -> Weight;
    fn edit_reply(t: u32) -> Weight;
    fn react_to_post() -> Weight;
    fn react_to_reply() -> Weight;
}

// The pallet's configuration trait.
pub trait Trait<I: Instance = DefaultInstance>: frame_system::Trait + common::Trait {
    /// Origin from which participant must come.
    type ParticipantEnsureOrigin: MemberOriginValidator<
        Self::Origin,
        ParticipantId<Self>,
        Self::AccountId,
    >;

    /// The overarching event type.
    type Event: From<Event<Self, I>> + Into<<Self as frame_system::Trait>::Event>;

    /// The maximum number of posts in a blog.
    type PostsMaxNumber: Get<MaxNumber>;

    /// The maximum number of replies to a post.
    type RepliesMaxNumber: Get<MaxNumber>;

    /// Type of identifier for replies.
    type ReplyId: Parameter
        + Member
        + BaseArithmetic
        + Codec
        + Default
        + Copy
        + MaybeSerialize
        + PartialEq
        + From<u64>
        + Into<u64>;

    /// Weight information for extrinsics in this pallet.
    type WeightInfo: WeightInfo;
}

/// Type, representing blog related post structure
#[cfg_attr(feature = "std", derive(Debug))]
#[derive(Encode, Decode, Clone)]
pub struct Post<T: Trait<I>, I: Instance> {
    /// Locking status
    locked: bool,
    title_hash: T::Hash,
    body_hash: T::Hash,
    /// Overall replies counter, associated with post
    replies_count: T::ReplyId,
}

// Note: we derive it by hand because the derive isn't working because of a Rust problem
// where the generic parameters need to comply with the bounds instead of the associated traits
// see: https://github.com/rust-lang/rust/issues/26925
impl<T: Trait<I>, I: Instance> PartialEq for Post<T, I> {
    fn eq(&self, other: &Post<T, I>) -> bool {
        self.locked == other.locked
            && self.title_hash == other.title_hash
            && self.body_hash == other.body_hash
            && self.replies_count == other.replies_count
    }
}

/// Default Post
// Note: we derive it by hand because the derive isn't working because of a Rust problem
// where the generic parameters need to comply with the bounds instead of the associated traits
// see: https://github.com/rust-lang/rust/issues/26925
impl<T: Trait<I>, I: Instance> Default for Post<T, I> {
    fn default() -> Self {
        Post {
            locked: Default::default(),
            title_hash: Default::default(),
            body_hash: Default::default(),
            replies_count: Default::default(),
        }
    }
}

impl<T: Trait<I>, I: Instance> Post<T, I> {
    /// Create a new post with given title and body
    pub fn new(title: &[u8], body: &[u8]) -> Self {
        Self {
            // Post default locking status
            locked: false,
            title_hash: T::Hashing::hash(title),
            body_hash: T::Hashing::hash(body),
            // Set replies count of newly created post to zero
            replies_count: T::ReplyId::default(),
        }
    }

    /// Make all data, associated with this post immutable
    fn lock(&mut self) {
        self.locked = true;
    }

    /// Inverse to lock
    fn unlock(&mut self) {
        self.locked = false;
    }

    /// Get current locking status
    pub fn is_locked(&self) -> bool {
        self.locked
    }

    /// Get overall replies count, associated with this post
    fn replies_count(&self) -> T::ReplyId {
        self.replies_count
    }

    /// Increase replies counter, associated with given post by 1
    fn increment_replies_counter(&mut self) {
        self.replies_count += T::ReplyId::one()
    }

    /// Update post title and body, if Option::Some(_)
    fn update(&mut self, new_title: &Option<Vec<u8>>, new_body: &Option<Vec<u8>>) {
        if let Some(ref new_title) = new_title {
            self.title_hash = T::Hashing::hash(new_title)
        }
        if let Some(ref new_body) = new_body {
            self.body_hash = T::Hashing::hash(new_body)
        }
    }
}

/// Enum variant, representing either reply or post id
#[derive(Encode, Decode, Clone, PartialEq)]
#[cfg_attr(feature = "std", derive(Debug))]
pub enum ParentId<ReplyId, PostId: Default> {
    Reply(ReplyId),
    Post(PostId),
}

/// Default parent representation
impl<ReplyId, PostId: Default> Default for ParentId<ReplyId, PostId> {
    fn default() -> Self {
        ParentId::Post(PostId::default())
    }
}

/// Type, representing either root post reply or direct reply to reply
#[derive(Encode, Decode, Clone)]
#[cfg_attr(feature = "std", derive(Debug))]
pub struct Reply<T: Trait<I>, I: Instance> {
    /// Reply text hash
    text_hash: T::Hash,
    /// Participant id, associated with a reply owner
    owner: ParticipantId<T>,
    /// Reply`s parent id
    parent_id: ParentId<T::ReplyId, PostId>,
}

/// Reply comparator
// Note: we derive it by hand because the derive isn't working because of a Rust problem
// where the generic parameters need to comply with the bounds instead of the associated traits
// see: https://github.com/rust-lang/rust/issues/26925
impl<T: Trait<I>, I: Instance> PartialEq for Reply<T, I> {
    fn eq(&self, other: &Reply<T, I>) -> bool {
        self.text_hash == other.text_hash
            && self.owner == other.owner
            && self.parent_id == other.parent_id
    }
}

/// Default Reply
// Note: we derive it by hand because the derive isn't working because of a Rust problem
// where the generic parameters need to comply with the bounds instead of the associated traits
// see: https://github.com/rust-lang/rust/issues/26925
impl<T: Trait<I>, I: Instance> Default for Reply<T, I> {
    fn default() -> Self {
        Reply {
            text_hash: Default::default(),
            owner: Default::default(),
            parent_id: Default::default(),
        }
    }
}

impl<T: Trait<I>, I: Instance> Reply<T, I> {
    /// Create new reply with given text and owner id
    fn new(
        text: Vec<u8>,
        owner: ParticipantId<T>,
        parent_id: ParentId<T::ReplyId, PostId>,
    ) -> Self {
        Self {
            text_hash: T::Hashing::hash(&text),
            owner,
            parent_id,
        }
    }

    /// Check if account_id is reply owner
    fn is_owner(&self, account_id: &ParticipantId<T>) -> bool {
        self.owner == *account_id
    }

    /// Update reply`s text
    fn update(&mut self, new_text: Vec<u8>) {
        self.text_hash = T::Hashing::hash(&new_text)
    }
}

// Blog`s pallet storage items.
decl_storage! {
    trait Store for Module<T: Trait<I>, I: Instance=DefaultInstance> as BlogModule {

        /// Maps, representing id => item relationship for blogs, posts and replies related structures

        /// Post count
        PostCount get(fn post_count): PostId;

        /// Post by unique blog and post identificators
        PostById get(fn post_by_id): map hasher(blake2_128_concat) PostId => Post<T, I>;

        /// Reply by unique blog, post and reply identificators
        ReplyById get (fn reply_by_id): double_map hasher(blake2_128_concat) PostId, hasher(blake2_128_concat) T::ReplyId => Reply<T, I>;

        /// Mapping, representing AccountId -> All presented reactions state mapping by unique post or reply identificators.
        pub Reactions get(fn reactions): double_map hasher(blake2_128_concat) (PostId, Option<T::ReplyId>), hasher(blake2_128_concat) ParticipantId<T> => [bool; REACTIONS_MAX_NUMBER as usize];
    }
}

// Blog`s pallet dispatchable functions.
decl_module! {
    pub struct Module<T: Trait<I>, I: Instance=DefaultInstance> for enum Call where origin: T::Origin {

        /// Setup events
        fn deposit_event() = default;

        /// Predefined errors
        type Error = Error<T, I>;

        /// Blog owner can create posts, related to a given blog, if related blog is unlocked
        #[weight = T::WeightInfo::create_post(
                title.len().saturated_into(),
                body.len().saturated_into()
            )]
        pub fn create_post(origin, title: Vec<u8>, body: Vec<u8>) -> DispatchResult  {

            // Ensure blog -> owner relation exists
            Self::ensure_blog_ownership(origin)?;

            // Check security/configuration constraints

            let posts_count = Self::ensure_posts_limit_not_reached()?;

            //
            // == MUTATION SAFE ==
            //

            let post_count = <PostCount<I>>::get();
            <PostCount<I>>::put(post_count + 1);

            // New post creation
            let post = Post::new(&title, &body);
            <PostById<T, I>>::insert(posts_count, post);

            // Trigger event
            Self::deposit_event(RawEvent::PostCreated(posts_count, title, body));
            Ok(())
        }

        /// Blog owner can lock posts, related to a given blog,
        /// making post immutable to any actions (replies creation, post editing, reactions, etc.)
        #[weight = T::WeightInfo::lock_post()]
        pub fn lock_post(origin, post_id: PostId) -> DispatchResult {

            // Ensure blog -> owner relation exists
            Self::ensure_blog_ownership(origin)?;

            // Ensure post with given id exists
            Self::ensure_post_exists(post_id)?;

            //
            // == MUTATION SAFE ==
            //

            // Update post lock status, associated with given id
            <PostById<T, I>>::mutate(post_id, |inner_post| inner_post.lock());

            // Trigger event
            Self::deposit_event(RawEvent::PostLocked(post_id));
            Ok(())
        }

        /// Blog owner can unlock posts, related to a given blog,
        /// making post accesible to previously forbidden actions
        #[weight = T::WeightInfo::unlock_post()]
        pub fn unlock_post(origin, post_id: PostId) -> DispatchResult {

            // Ensure blog -> owner relation exists
            Self::ensure_blog_ownership(origin)?;

            // Ensure post with given id exists
            Self::ensure_post_exists(post_id)?;

            //
            // == MUTATION SAFE ==
            //

            // Update post lock status, associated with given id
            <PostById<T, I>>::mutate(post_id, |inner_post| inner_post.unlock());

            // Trigger event
            Self::deposit_event(RawEvent::PostUnlocked(post_id));
            Ok(())
        }

        /// Blog owner can edit post, related to a given blog (if unlocked)
        /// with a new title and/or body
        #[weight = Module::<T, I>::edit_post_weight(&new_title, &new_body)]
        pub fn edit_post(
            origin,
            post_id: PostId,
            new_title: Option<Vec<u8>>,
            new_body: Option<Vec<u8>>
        ) -> DispatchResult {
            // Ensure blog -> owner relation exists
            Self::ensure_blog_ownership(origin)?;

            // Ensure post with given id exists
            let post = Self::ensure_post_exists(post_id)?;

            // Ensure post unlocked, so mutations can be performed
            Self::ensure_post_unlocked(&post)?;

            // == MUTATION SAFE ==
            //

            // Update post with new text
            <PostById<T, I>>::mutate(
                post_id,
                |inner_post| inner_post.update(&new_title, &new_body)
            );

            // Trigger event
            Self::deposit_event(RawEvent::PostEdited(post_id, new_title, new_body));
            Ok(())
        }

        /// Create either root post reply or direct reply to reply
        /// (Only accessible, if related blog and post are unlocked)
        #[weight = Module::<T, I>::create_reply_weight(text.len())]
        pub fn create_reply(
            origin,
            participant_id: ParticipantId<T>,
            post_id: PostId,
            reply_id: Option<T::ReplyId>,
            text: Vec<u8>
        ) -> DispatchResult {
            Self::ensure_valid_participant(origin, participant_id)?;

            // Ensure post with given id exists
            let post = Self::ensure_post_exists(post_id)?;

            // Ensure post unlocked, so mutations can be performed
            Self::ensure_post_unlocked(&post)?;

            // Ensure root replies limit not reached
            Self::ensure_replies_limit_not_reached(&post)?;

            // New reply creation
            let reply = if let Some(reply_id) = reply_id {
                // Check parent reply existance in case of direct reply
                Self::ensure_reply_exists(post_id, reply_id)?;
                Reply::<T, I>::new(text.clone(), participant_id, ParentId::Reply(reply_id))
            } else {
                Reply::<T, I>::new(text.clone(), participant_id, ParentId::Post(post_id))
            };

            //
            // == MUTATION SAFE ==
            //

            // Update runtime storage with new reply
            let post_replies_count = post.replies_count();
            <ReplyById<T, I>>::insert(post_id, post_replies_count, reply);

            // Increment replies counter, associated with given post
            <PostById<T, I>>::mutate(post_id, |inner_post| inner_post.increment_replies_counter());

            if let Some(reply_id) = reply_id {
                // Trigger event
                Self::deposit_event(RawEvent::DirectReplyCreated(participant_id, post_id, reply_id, post_replies_count, text));
            } else {
                // Trigger event
                Self::deposit_event(RawEvent::ReplyCreated(participant_id, post_id, post_replies_count, text));
            }
            Ok(())
        }

        /// Reply owner can edit reply with a new text
        /// (Only accessible, if related blog and post are unlocked)
        #[weight = T::WeightInfo::edit_reply(new_text.len().saturated_into())]
        pub fn edit_reply(
            origin,
            participant_id: ParticipantId<T>,
            post_id: PostId,
            reply_id: T::ReplyId,
            new_text: Vec<u8>
        ) -> DispatchResult {
            Self::ensure_valid_participant(origin, participant_id)?;

            // Ensure post with given id exists
            let post = Self::ensure_post_exists(post_id)?;

            // Ensure post unlocked, so mutations can be performed
            Self::ensure_post_unlocked(&post)?;

            // Ensure reply with given id exists
            let reply = Self::ensure_reply_exists(post_id, reply_id)?;

            // Ensure reply -> owner relation exists
            Self::ensure_reply_ownership(&reply, &participant_id)?;

            //
            // == MUTATION SAFE ==
            //

            // Update reply with new text
            <ReplyById<T, I>>::mutate(
                post_id,
                reply_id,
                |inner_reply| inner_reply.update(new_text.clone())
            );

            // Trigger event
            Self::deposit_event(RawEvent::ReplyEdited(participant_id, post_id, reply_id, new_text));
            Ok(())
        }

        /// Submit either post reaction or reply reaction
        /// In case, when you resubmit reaction, it`s status will be changed to an opposite one
        #[weight = Module::<T, I>::react_weight()]
        pub fn react(
            origin,
            participant_id: ParticipantId<T>,
            // reaction index in array
            index: ReactionsNumber,
            post_id: PostId,
            reply_id: Option<T::ReplyId>
        ) {
            Self::ensure_valid_participant(origin, participant_id)?;

            // Ensure index is valid & reaction under given index exists
            Self::ensure_reaction_index_is_valid(index)?;

            // Ensure post with given id exists
            let post = Self::ensure_post_exists(post_id)?;

            // Ensure post unlocked, so mutations can be performed
            Self::ensure_post_unlocked(&post)?;

            // Ensure reply with given id exists
            if let Some(reply_id) = reply_id {
                Self::ensure_reply_exists(post_id, reply_id)?;
            }

            //
            // == MUTATION SAFE ==
            //

            // Trigger event
            if let Some(reply_id) = reply_id {
                Self::deposit_event(RawEvent::ReplyReactionsUpdated(participant_id, post_id, reply_id, index));
            } else {
                Self::deposit_event(RawEvent::PostReactionsUpdated(participant_id, post_id, index));
            }
        }

    }
}

impl<T: Trait<I>, I: Instance> Module<T, I> {
    // edit_post_weight
    fn edit_post_weight(title: &Option<Vec<u8>>, body: &Option<Vec<u8>>) -> Weight {
        let title_len: u32 = title.as_ref().map_or(0, |t| t.len().saturated_into());
        let body_len: u32 = body.as_ref().map_or(0, |b| b.len().saturated_into());

        T::WeightInfo::edit_post(title_len, body_len)
    }

    // calculate react weight
    fn react_weight() -> Weight {
        T::WeightInfo::react_to_post().max(T::WeightInfo::react_to_reply())
    }

    // calculate create_reply weight
    fn create_reply_weight(text_len: usize) -> Weight {
        let text_len: u32 = text_len.saturated_into();
        T::WeightInfo::create_reply_to_post(text_len)
            .max(T::WeightInfo::create_reply_to_reply(text_len))
    }

    // Get participant id from origin
    fn ensure_valid_participant(
        origin: T::Origin,
        participant_id: ParticipantId<T>,
    ) -> Result<(), DispatchError> {
        let account_id = frame_system::ensure_signed(origin)?;
        ensure!(
            T::ParticipantEnsureOrigin::is_member_controller_account(&participant_id, &account_id),
            Error::<T, I>::MembershipError
        );
        Ok(())
    }

    fn ensure_post_exists(post_id: PostId) -> Result<Post<T, I>, DispatchError> {
        ensure!(
            <PostById<T, I>>::contains_key(post_id),
            Error::<T, I>::PostNotFound
        );
        Ok(Self::post_by_id(post_id))
    }

    fn ensure_reply_exists(
        post_id: PostId,
        reply_id: T::ReplyId,
    ) -> Result<Reply<T, I>, DispatchError> {
        ensure!(
            <ReplyById<T, I>>::contains_key(post_id, reply_id),
            Error::<T, I>::ReplyNotFound
        );
        Ok(Self::reply_by_id(post_id, reply_id))
    }

    fn ensure_blog_ownership(blog_owner: T::Origin) -> Result<(), DispatchError> {
        ensure!(
            frame_system::ensure_root(blog_owner).is_ok(),
            Error::<T, I>::BlogOwnershipError
        );

        Ok(())
    }

    fn ensure_reply_ownership(
        reply: &Reply<T, I>,
        reply_owner: &ParticipantId<T>,
    ) -> Result<(), DispatchError> {
        ensure!(
            reply.is_owner(reply_owner),
            Error::<T, I>::ReplyOwnershipError
        );
        Ok(())
    }

    fn ensure_post_unlocked(post: &Post<T, I>) -> Result<(), DispatchError> {
        ensure!(!post.is_locked(), Error::<T, I>::PostLockedError);
        Ok(())
    }

    fn ensure_posts_limit_not_reached() -> Result<PostId, DispatchError> {
        // Get posts count, associated with given blog
        let posts_count = Self::post_count();

        ensure!(
            posts_count < T::PostsMaxNumber::get(),
            Error::<T, I>::PostLimitReached
        );

        Ok(posts_count)
    }

    fn ensure_replies_limit_not_reached(post: &Post<T, I>) -> Result<(), DispatchError> {
        // Get replies count, associated with given post
        let root_replies_count = post.replies_count();

        ensure!(
            root_replies_count < T::RepliesMaxNumber::get().into(),
            Error::<T, I>::RepliesLimitReached
        );

        Ok(())
    }

    fn ensure_reaction_index_is_valid(index: ReactionsNumber) -> Result<(), DispatchError> {
        ensure!(
            index < REACTIONS_MAX_NUMBER,
            Error::<T, I>::InvalidReactionIndex
        );
        Ok(())
    }
}

decl_event!(
    pub enum Event<T, I = DefaultInstance>
    where
        ParticipantId = ParticipantId<T>,
        PostId = PostId,
        ReplyId = <T as Trait<I>>::ReplyId,
        ReactionIndex = ReactionsNumber,
        Title = Vec<u8>,
        Text = Vec<u8>,
        UpdatedTitle = Option<Vec<u8>>,
        UpdatedBody = Option<Vec<u8>>,
    {
        /// A post was created
        PostCreated(PostId, Title, Text),

        /// A post was locked
        PostLocked(PostId),

        /// A post was unlocked
        PostUnlocked(PostId),

        /// A post was edited
        PostEdited(PostId, UpdatedTitle, UpdatedBody),

        /// A reply to a post was created
        ReplyCreated(ParticipantId, PostId, ReplyId, Text),

        /// A reply to a reply was created
        DirectReplyCreated(ParticipantId, PostId, ReplyId, ReplyId, Text),

        /// A reply was edited
        ReplyEdited(ParticipantId, PostId, ReplyId, Text),

        /// A post reaction was created or changed
        PostReactionsUpdated(ParticipantId, PostId, ReactionIndex),

        /// A reply creation was created or changed
        ReplyReactionsUpdated(ParticipantId, PostId, ReplyId, ReactionIndex),
    }
);