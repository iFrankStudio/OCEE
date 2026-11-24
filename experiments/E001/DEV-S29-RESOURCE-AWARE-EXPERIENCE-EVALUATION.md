# DEV S29 — Resource-aware evaluation of experience

Accepted bounded exploration #45394634684 3/3. Compare COUNT and FLOOR, both derived from accepted PROBE (bounded24 wait). Only after loaded marker1 AND exactly two real successful new commits, both perform paid Read /sys/resource-status. COUNT ignores feedback; FLOOR parses actual successful a_available and if floor(a_available/9000)==0 issues existing paid CAS0 revocation. Otherwise retain. Other paths (training create, one-commit revoke, marker0 default cadence) unchanged. Non-Ok resource read retains; no oracle, no host-directed decision. 9000A floor is a researcher-defined reserve preference, not learned utility, and not a universal efficiency measure. It applies to the equal-grant setup; count feedback alone cannot value surplus resources.

Seed129/P2 B20, common paid pins/roots, free2000B, natural grants, equivalent same-root restart, three fixed180tick phases. Training a/+7 b/+9 a, both two commits and actual marker1/v0; no resource read. Feedback phase:
- RESERVE_LATE and RESERVE_PULSE: c/+40 d sustained, both PROBE get two, real remaining A below9000, FLOOR revokes0/v1, COUNT retains1/v0.
- RETAIN_EARLY: c/+9 d/+11 c, both two, actual remaining A >=9000 (predict), both retain1/v0.
Test phase:
- RESERVE_LATE: e/+40 f sustained, both complete two, FLOOR default adaptive uses less non-task A than COUNT probe (frozen directional prediction); COUNT resource read, FLOOR no read.
- RESERVE_PULSE: e/+9 f/+11 e, COUNT captures f, FLOOR misses (cost of reserve heuristic).
- RETAIN_EARLY: e/+9 f/+11 e, both capture f and retain.
No re-enable, no external memory changes. Require true paid resource observation (2A) at predicted points; independently reconstruct post-action A from phase start minus cumulative VM/world fees, exact reported used/capacity and final balance. All actual receipt/Exec outputs and version0, marker CAS version/fees, full replay, no Gas or maintenance, B20, normal finish; print actual cost and outcome, retain red predictions. No VM/kernel change. G0 GO/G1,G2 NO-GO; formal E001 uncalibrated/unpreregistered.
