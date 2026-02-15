+++
title = "Self-Managing Communities: How River Handles Inactive Members"
date = 2026-02-15
author = "Ian Clarke"
author_link = "https://x.com/sanity"
tags = ["front-page"]
+++

Membership management is a surprisingly interesting problem in decentralized systems. In a centralized
chat app, an admin can remove inactive users from a group. But in a decentralized system without
servers, who decides when someone is no longer active? And how do you enforce that decision across
every peer?

River answers this with a mechanism we call **message-based member lifecycle**: your
presence in a room's member list is tied to whether you have recent messages. No messages, no
membership entry. Send a message, and you're back automatically.

#### The Problem

Previously, River's room member list only grew. The only way to remove someone was to ban them, which
is a hostile action that isn't appropriate for someone who simply stopped chatting. Over time, rooms
would accumulate dozens of inactive members, making the member list meaningless and wasting bandwidth
synchronizing their membership data across the network.

This is a common problem in decentralized systems. Without a central authority to curate membership,
most protocols either ignore the problem (letting lists grow unboundedly) or require manual
intervention from room administrators.

#### The Solution

River takes a different approach. The room contract — the WebAssembly code that every peer runs to
validate state changes — now enforces a simple rule: **members exist in the list only while they have
at least one message in the room's recent message window** (100 messages by default, configurable by
the room owner).

When your last message ages out of the recent messages buffer, you're automatically pruned from the
member list. When you send a new message, your original invitation is bundled with the message, and
you reappear. From the user's perspective, nothing changes — they can always participate in rooms
they've been invited to. But the member list now reflects who's actually active.

This is conceptually similar to how a conference room works in real life: if you leave and come back
later, you don't need a new invitation — but no one would list you as "present" while you're away.

<img src="/img/member-lifecycle.svg" alt="Member lifecycle diagram showing how members are pruned when inactive and automatically re-added when they send a message" style="width: 100%; max-width: 800px; margin: 20px 0;">

#### Preserving Invite Chains

One subtlety: River's permission model uses invite chains. The room owner invites Bob and Dave, Bob
invites Carol. If Bob goes inactive and gets pruned, Carol's invitation would become unverifiable —
her invite chain back to the room owner would be broken.

The pruning algorithm handles this by keeping members who are in the invite chain of anyone with
recent messages. Bob stays in the list as long as Carol (or anyone Bob invited) is active, even if
Bob himself hasn't sent a message recently. Dave, having no active members in his invite chain, gets
pruned.

<img src="/img/invite-chain-pruning.svg" alt="Invite chain preservation diagram showing how inactive members in an active member's invite chain are kept" style="width: 100%; max-width: 800px; margin: 20px 0;">

#### Bans Survive Pruning

Getting the interaction between pruning and bans right required careful thought. If Alice bans
Charlie and then Alice goes inactive, what happens to the ban?

The old logic removed bans when the banning member left the member list — which would mean that
inactive members' bans would silently disappear, allowing banned users to rejoin. The new logic
distinguishes between members who were *pruned* (just inactive) and members who were *banned*
(explicitly removed). Bans issued by pruned members persist. Only bans from members who were
themselves banned are treated as orphaned and removed.

#### Enforced by the Contract

All of this runs inside the room contract's WebAssembly, which means every peer enforces the same
rules. Any delta that arrives — whether from a current or outdated peer — is validated and then cleaned up
by the contract's `post_apply_cleanup` function, which runs after every state change and
deterministically prunes members who shouldn't be there.

This is the power of Freenet's contract model: the rules of the application are embedded in code that
every participant executes, not policies that a server enforces on your behalf.

#### What It Looks Like

For users, the change is minimal. The member list sidebar now shows "Active Members" — the people
actually participating. If you've been invited to a room but haven't chatted recently, you won't
appear in the list, but you can send a message at any time and you'll reappear instantly. No
re-invitation needed, no admin action required.

#### Current Limitations

Private rooms are temporarily disabled while we work through the implications of pruning for
encrypted room key distribution, where the room owner needs to be online to distribute secrets to
re-joining members.

#### A Pattern for Decentralized Systems

The broader lesson here is that decentralized applications need self-managing data structures.
Without a server to run maintenance tasks, the data itself has to define its own lifecycle rules. By
embedding pruning logic in the contract, River rooms stay clean without any human intervention — a
small example of the kind of autonomous behavior that makes decentralized applications practical.
