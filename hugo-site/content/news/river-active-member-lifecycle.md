+++
title = "Self-Managing Communities: How River Handles Inactive Members"
date = 2026-02-15
author = "Ian Clarke"
author_link = "https://x.com/sanity"
tags = ["front-page"]
+++

Membership management is a deceptively hard problem in decentralized systems. In a centralized chat
app, an admin can remove inactive users from a group. But in a decentralized system without servers,
who decides when someone is no longer active? How do you enforce that decision across every peer
without coordination? And how do you do it without clocks, which are notoriously unreliable in
distributed systems?

River's answer is **message-based member lifecycle**: your presence in a room's member list is tied
to whether you have recent messages. No messages, no membership entry. Send a message, and you're
back automatically. It's a mechanism that falls naturally out of the constraints of decentralized
systems — and it turns out to be a better model than what centralized apps do.

#### The Problem

Previously, River's room member list only grew. The only way to remove someone was to ban them, which
is a hostile action that isn't appropriate for someone who simply stopped chatting. Over time, rooms
accumulate dozens of inactive members, making the member list meaningless and wasting bandwidth
synchronizing their membership data across the network.

This is a common problem in decentralized systems. Without a central authority to curate membership,
most protocols either ignore the problem (letting lists grow unboundedly) or require manual
intervention from room administrators. Neither scales.

#### Why Not Use Timestamps?

The obvious approach is to prune members who haven't been active for some period — say, 30 days.
This is what centralized systems do. But it has a fundamental problem in a decentralized context:
**peers don't share a reliable clock.**

Timestamps in distributed systems are unreliable. Peers can have skewed clocks, and there's no
authority to adjudicate disagreements. If Alice's clock is a month ahead, she'd see Bob as inactive
when he isn't. If Bob's clock is behind, his "recent" messages would look old to everyone else. You
end up needing a consensus mechanism just to agree on what time it is — which is a heavyweight
solution for a simple membership problem.

A heartbeat-based approach (periodic "I'm here" messages) avoids the clock problem but creates
bandwidth overhead proportional to the number of members, even when nobody is actually chatting. It
also requires peers to be online to send heartbeats, which conflicts with the reality of mobile and
intermittently-connected devices.

#### The Solution: Messages as Proof of Presence

River takes a different approach. The room contract — the WebAssembly code that every peer executes
to validate state changes — enforces a simple rule: **members exist in the list only while they have
at least one message in the room's recent message window.**

The message window defaults to 100 messages and is configurable by the room owner. When your last
message scrolls out of that window, you're automatically pruned from the member list. When you send a
new message, your original invitation is bundled with the message delta, and you reappear. From the
user's perspective, nothing changes — they can always participate in rooms they've been invited to.
But the member list reflects who's actually active.

This works because messages are the one thing peers already agree on — they're part of the shared
state. No additional protocol, no clocks, no heartbeats. The membership list becomes a **derived
property** of the message history rather than an independent data structure to maintain.

It's conceptually similar to how a conference room works in real life: if you leave and come back
later, you don't need a new invitation — but no one would list you as "present" while you're away.

<img src="/img/member-lifecycle.svg" alt="Member lifecycle diagram showing how members are pruned when inactive and automatically re-added when they send a message" style="width: 100%; max-width: 800px; margin: 20px 0;">

#### Why 100 Messages?

The default window size of 100 messages is a trade-off between three concerns:

- **Too small** (e.g. 10): Members get pruned quickly in active rooms, leading to frequent
  prune/rejoin cycles and unnecessary bandwidth from re-transmitting invite chains.
- **Too large** (e.g. 10,000): The member list would grow toward the old unbounded behavior,
  defeating the purpose. The message buffer itself would also consume significant bandwidth during
  synchronization.
- **100 messages** strikes a practical balance: in a moderately active room, this represents roughly
  a day or two of conversation. Members who haven't spoken in that time are likely genuinely
  inactive.

Room owners can tune this via the room configuration. A high-traffic announcement channel might use a
smaller window; a slow-moving coordination group might use a larger one.

#### Deterministic Convergence

A critical requirement: all peers must converge to the **same** member list, regardless of the order
they receive messages. If Alice receives Bob's message before Carol's, and Dave receives them in the
opposite order, they must still end up with identical member lists.

River achieves this through the contract's `post_apply_cleanup` function, which runs after every
state change. It scans the current message window, collects the set of message authors, walks their
invite chains, and retains only the members who are needed. The result is then sorted
deterministically by member ID. Because the function operates on the same inputs (the converged
message list) regardless of how those messages arrived, every peer reaches the same output.

This is a property of the CRDT (Conflict-free Replicated Data Type) design: the merge operation is
commutative and idempotent, so message ordering doesn't affect the final state.

#### Preserving Invite Chains

One subtlety: River's permission model uses invite chains. The room owner invites Bob and Dave, Bob
invites Carol. If Bob goes inactive and gets pruned, Carol's invitation would become unverifiable —
her invite chain back to the room owner would be broken.

The pruning algorithm handles this by keeping members who are in the invite chain of anyone with
recent messages. Bob stays in the list as long as Carol (or anyone Bob invited) is active, even if
Bob himself hasn't sent a message recently. Dave, having no active invitees in his chain, gets
pruned normally.

This creates an interesting emergent property: "connectors" — members who invited many active users —
persist in the list even without posting, because they're structurally necessary. This roughly
mirrors real social dynamics, where a person who brought a group together remains relevant even if
they go quiet.

<img src="/img/invite-chain-pruning.svg" alt="Invite chain preservation diagram showing how inactive members in an active member's invite chain are kept" style="width: 100%; max-width: 800px; margin: 20px 0;">

#### Bans Survive Pruning

Getting the interaction between pruning and bans right required careful thought and we got it wrong on
the first try.

If Alice bans Charlie and then Alice goes inactive, what happens to the ban? The original logic
removed bans when the banning member left the member list — which meant that inactive members' bans
would silently disappear, allowing banned users to rejoin. We caught this during testing when a
banned user reappeared after the banner went inactive.

The fix required distinguishing between two reasons a member might leave the list:

- **Pruned** (inactive): The member's bans persist. They're still a legitimate member of the
  community; they just haven't spoken recently.
- **Banned** (explicitly removed): The member's bans become orphaned and are cleaned up. A banned
  member's authority is revoked, including any bans they issued.

This distinction matters because it preserves the intent of moderation actions. A ban is a deliberate
community decision that should survive the banner going AFK.

#### What Happens During Network Partitions?

In a decentralized system, peers can temporarily lose connectivity and accumulate divergent state. What
happens when they reconnect?

Each peer independently applies the same pruning rules to its local state. When peers sync, they
merge their message lists (keeping all unique messages up to the buffer limit) and then re-run
`post_apply_cleanup`. Because pruning is derived from the merged message list — not from each peer's
individual pruning decisions — the result converges correctly.

A concrete scenario: Alice is offline for a day. Her peer has an older message list. When she
reconnects, her peer receives the latest messages from the network, merges them with her local state,
and prunes accordingly. She doesn't need to "catch up" on pruning decisions — she just needs the
current messages, and the pruning falls out deterministically.

The same applies to conflicting membership changes: if two peers independently prune different
members, the merged state will prune based on the combined message history, which is the correct
behavior.

#### Cost and Performance

Pruning adds computational work to every state merge, but the cost is modest:

- **Scanning messages for authors**: Linear in the message buffer size (default 100).
- **Walking invite chains**: Linear in the number of members times chain depth.
- **Sorting the member list**: O(n log n) in the member count.

With the defaults (100 messages, max 200 members), this takes microseconds on typical hardware —
negligible compared to CBOR serialization and network latency.

The more significant impact is on **bandwidth**. Each member adds roughly 200–300 bytes of
serialized data (public key, invitation signature, member info). At 200 members, that's ~50KB per
full state sync. Pruning inactive members directly reduces this. In a room where only 20 of 200
invited members are active, pruning reduces the synchronized member data by 90%.

When a pruned member rejoins, their message delta includes their original `AuthorizedMember` entry
and their full invite chain — a one-time cost of a few hundred bytes. This is significantly cheaper
than maintaining their membership data in every sync while they're inactive.

#### User Experience

From the user's perspective, the most visible change is that the member list sidebar now shows
"Active Members" — the people currently participating. If you've been invited to a room but haven't
chatted recently, you won't appear in the list.

When a pruned user opens the room and sends a message, they reappear instantly. No re-invitation
needed, no admin action required. The UI caches their original invitation and nickname locally, so
the rejoin is seamless — they don't lose their display name or need to reconfigure anything.

There is an intentional design choice here: **we don't notify users when they're pruned.** Being
pruned isn't a punishment or an event — it's the natural state of not participating. Just as you
wouldn't expect a notification for leaving a conference room by walking away, you shouldn't be
alarmed by not appearing in the active list of a room you haven't used recently.

#### Limitations and Open Questions

**Private rooms and secret distribution.** In private rooms, messages are encrypted with a shared
secret that the room owner distributes to members. When a member is pruned, their encrypted secret
copy is cleaned up. When they rejoin, the owner needs to re-distribute the secret — which requires
the owner to be online. We're working on approaches to handle this, including pre-cached secrets and
peer-assisted distribution.

**Lurkers.** Some users want to read without posting. Under the current design, they'll be pruned
from the member list, though they can still read messages (the room state is still synchronized to
their peer). This is arguably correct — a read-only user isn't an "active member" — but it may feel
surprising. Future iterations could introduce a lightweight presence mechanism for users who want to
remain visible without posting, but we'd need to solve the heartbeat bandwidth problem mentioned
earlier.

**High-churn rooms.** In a room with very high message volume and many participants, the prune/rejoin
cycle could create noticeable churn in the member list. The message window size mitigates this, but
rooms with hundreds of active members may need larger windows, which increases sync bandwidth. The
`max_members` configuration (default 200) provides a hard cap, after which the contract prunes
members with the longest invite chains first.

**Invite chain depth.** Deep invite chains (A invited B who invited C who invited D...) create a
structural dependency: pruning A cascades to B, C, and D unless they have independent paths to the
owner. In practice, most rooms have shallow chains (1-2 levels deep), but this could be an issue in
large communities with deep delegation hierarchies.

#### Enforced by the Contract

All of this runs inside the room contract's WebAssembly, which means every peer enforces the same
rules. Deltas arriving from any peer — current or outdated — are validated and then cleaned up by the
contract's `post_apply_cleanup` function. No peer can opt out of pruning or maintain a stale member
list.

This is the key architectural insight: by embedding lifecycle rules in the contract rather than
treating them as optional policy, the system is self-maintaining. Every state transition is an
opportunity to clean up, and the cleanup is deterministic.

#### A Pattern for Decentralized Systems

The broader lesson is that decentralized applications need **self-managing data structures**. Without
a server to run maintenance tasks, the data itself has to define its own lifecycle rules. The
alternative — hoping that human administrators will manually curate state across a peer-to-peer
network — doesn't scale.

Message-based lifecycle is one instance of a general principle: **derive structure from the data you
already have, rather than maintaining separate metadata that can drift out of sync.** The member list
isn't a separate thing to manage; it's a view over the message history. This eliminates an entire
class of consistency problems.

We expect this pattern — contracts that enforce their own housekeeping as part of every state
transition — to be common across Freenet applications. River is just the first example.
