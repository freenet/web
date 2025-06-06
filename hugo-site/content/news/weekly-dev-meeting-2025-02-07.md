+++
tags = ["dev-meeting", "front-page"]
title = "Weekly Developer Meeting" 
date = 2025-02-07
+++

#### **Attendees**

- Ian Clarke
- Ignacio Duart

#### **Key Updates & Discussion Points**

1. **Network Connection Fixes**

   - Most issues preventing stable peer connections have been fixed.
   - The network can now maintain multiple connections, resolving a previous issue where only two
     peers could connect at a time.
   - The root cause was a combination of:
     - The gateway not waiting long enough before cleaning transient connections.
     - A logic bug in packet receipt handling that caused repeated transmission loops.
   - A fix was implemented to send receipts after a time threshold, even if the packet count limit
     wasn't met.

2. **Remaining Network Issue**

   - There is still a logical issue in peer filtering when forwarding new connection requests.
   - Some peers are mistakenly being filtered out, preventing them from establishing connections.
   - Ignacio believes the issue stems from unintended state mutations in transaction forwarding
     logic.

3. **Ping Contract Debugging**

   - The team was testing the ping contract using a single peer for writes and others for reads.
   - An issue was discovered where state updates are not propagating, likely caused by a previous
     efficiency optimization that prevented redundant updates.
   - The get function may also not be retrieving the contract correctly.

4. **Persistent Contract Updates for Demo**

   - To ensure the contract is continuously updated, the team will run the ping contract on a
     gateway to keep activity visible.

5. **Code & Technical Adjustments**

   - Discussed removing an outdated connection filtering rule preventing multiple nodes on the same
     local network from connecting.
   - Ensured that the network can support multiple peers with the same location, using historical
     response times for routing decisions.

6. **River Chat Integration & Serialization Issue**
   - Ian is working on integrating River with Freenet.
   - Encountering a serialization error when sending messages via WebSockets.
   - Ignacio suggested debugging the contract execution process by adding logging inside the
     contract functions.
   - Suspected issue: The contract is failing to deserialize state data inside the WASM execution
     layer.

#### **Next Steps**

- **Ignacio** will continue debugging the contract issue and monitor network stability over the
  weekend.
- **Ian** will investigate the serialization error in River and implement contract logging to
  pinpoint the deserialization issue.
- **Both** will work toward a stable release early next week, with a recorded demo prepared by
  midweek.

#### **Closing Notes**

- Ignacio mentioned a tool for creating realistic AI-generated demo videos with avatars; he will
  follow up with Ian on its name.
