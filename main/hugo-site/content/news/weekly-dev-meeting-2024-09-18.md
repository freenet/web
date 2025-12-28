+++
tags = ["dev-meeting"]
title = "Weekly Dev Meeting" 
date = 2024-09-18
+++

### Key Achievements:

- **Transport Layer Improvements:** The network joining through a gateway works fine with most
  errors resolved. Nodes are acquiring connections, and the retry mechanism ensures successful
  connections even when initial attempts fail.
- **Unit Test Success:** Most transport layer unit tests are passing, with the system able to
  establish connections after retries. Random packet drop simulations highlight some intermittent
  failures, but overall functionality is stable.
- **Connection Debugging:** Logs show nodes progressively acquiring connections over time. The team
  is working on cleaning up the test environment for better debugging.
- **State Synchronization:** Currently, when peers update their state, the entire state is sent
  rather than just deltas. This approach is suboptimal, and the plan is to shift to delta updates
  after the initial release.

### Remaining Tasks:

1. **Finalizing the Gateway and Transport Logic:**

   - Confirm that all hops forward connections properly across the maximum number of peers.
   - Ensure that simulations consistently reach max connections and that no nodes get stuck due to
     filtered peers.
   - Clean up and merge the remaining pull requests related to transport and connection handling.

2. **UI and Simulation Enhancements:**

   - Refine the UI to allow easier debugging and visual tracking of operations like `put` and `get`.
   - Perform further tests with simulated events to validate the system’s behavior under real
     network conditions.

3. **Real-Network Testing:**

   - Conduct real-world tests by deploying the network on Ian's machine to confirm if nodes can join
     and function correctly.
   - Address any issues that arise during real-network testing.

4. **Delta Updates:**
   - Implement a state synchronization protocol where only deltas are sent between peers, reducing
     bandwidth and improving efficiency. This will be done shortly after the initial release.

### Conclusion:

The project is nearing completion, with only a few remaining tasks related to connection handling,
UI enhancements, and state synchronization. The team plans to release the new version soon and
follow up with subsequent updates to fix any emerging issues.
