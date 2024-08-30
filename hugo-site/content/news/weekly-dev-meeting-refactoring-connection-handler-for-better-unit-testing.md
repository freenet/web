+++
tags = ["dev-meeting", "front-page"]
title = "Weekly Dev Meeting - Friday, August 30th, 2024" 
date = 2024-08-30
+++

**Key Progress:**

- **Major Refactor Completed:**

  - Refactor focused on initial connections between nodes via the gateway.
  - Addressed numerous issues that were causing problems in the network.
  - Integration testing has been improved, leading to faster feedback for changes.

- **Integration Testing Improvements:**
  - Fixed various errors in integration code that were causing issues during testing.
  - Adjustments have sped up the testing process significantly, allowing for quicker iterations.

**Current Blockers:**

- **Transport Layer Issues:**
  - There is a problem with the handshake process between the gateway and peers.
  - The gateway and peers are not correctly syncing on the use of symmetric and asymmetric keys
    during communication.
  - Ignacio is currently diagnosing this issue, which appears to be the last major blocker.

**Other Developments:**

- **Ghost Keys:**

  - Discussion of "Ghost Keys," an independent system Ian has been working on, which focuses on
    anonymous, verifiable identities.
  - Ian mentioned the importance of naming, noting that "Ghost Keys" has resonated well with people
    and sparked interest.
  - The system is largely independent of Freenet but has been in discussion for some time.

- **Group Chat System:**
  - Ian has been developing a group chat system, with a focus on using macros to handle complex
    contract operations.
  - The discussion revealed challenges with field dependencies within the chat room contract,
    particularly regarding the correct application of deltas.
  - Ian is considering integrating elements from an existing experimental approach within Freenet’s
    standard library to streamline development and reduce redundancy.

**Next Steps:**

- **Focus on Transport Layer Fixes:**

  - Once the handshake issue is resolved, the network should be able to establish reliable
    connections.
  - After this, any remaining issues should be minor and more easily addressed.

- **Continue Development of Group Chat System:**
  - Ian will review existing experimental approaches to potentially integrate them into the group
    chat system.
  - The goal is to create a more robust and efficient contract framework that can serve as an
    example for more complex systems in the future.

**Conclusion:**

- The network is moving closer to a stable release.
- The team is hopeful that these final issues can be resolved quickly, allowing users to begin
  starting up nodes and participating in the network.
- Parallel developments, like the Ghost Keys system and the group chat project, are also
  progressing, contributing to the broader ecosystem Freenet aims to support.
