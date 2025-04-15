+++
tags = ["dev-meeting", "front-page"]
title = "Developer Meeting" 
date = 2025-04-10
+++

**Attendees:** Ian Clarke, Ignacio Duart

In this week’s core team meeting, we made significant progress on two fronts:

1. **Update Propagation Over the Network**  
   Updates are now successfully propagating across Freenet peers, a key milestone toward stable
   multi-node operation. Ignacio and Ector have been working together to finalize this, and although
   there may still be an edge case affecting update propagation through intermediary peers, once
   confirmed, we expect to release a new version shortly.

2. **Delegate API & River Integration**  
   Ian and Ignacio explored a subtle bug in River’s delegate integration. The issue stems from
   ambiguity in how application messages are processed and returned. Specifically, marking a
   delegate message as `processed` prevents it from being delivered to the application, even when a
   reply is expected. This led to a deeper discussion about improving the delegate runtime API by
   distinguishing between:

   - _Intermediate messages_ (e.g. `GetSecret`), which expect a reply from the node, and
   - _Terminal messages_, which are returned to the application.

   The current approach, which relies on a `process` flag, is prone to confusion and bugs. Ignacio
   proposed a cleaner design that uses distinct message types for intermediate and terminal steps.
   This change would clarify delegate behavior and reduce potential developer error.

   While this issue affects how delegate messages are handled internally, it **does not block** the
   ability to demonstrate **River working over the live Freenet network**. River already functions
   with a workaround, and delegates are primarily used for storing chat room metadata across
   refreshes.

3. **Executor Pool Refactor (Upcoming)**  
   Ignacio also has an efficiency-focused PR in progress that addresses long-running contracts. Once
   complete, it will allow better isolation of failing contracts by replacing execution workers
   without crashing the process.

---

**Next Steps:**

- Finalize testing of update propagation across multi-hop peer paths.
- Consider refactoring the delegate API to make message flow and processing states more explicit.
- Continue preparing for a public demonstration of River running on the live network.
