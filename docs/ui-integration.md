# verified-rcv UI integration spec

This document specifies the frontend integration for verified-rcv. The UI is **orthogonal to the verification trust chain** — it presents and orchestrates, but contributes no trust-bearing computation. The trust chain ends at chain state + enclave attestation; the UI just renders both.

**Spec version: aligned with intent v0.3.10 / commit `4cc50b0` (2026-05-26).** Major API breaks from the prior `DstackAttestation` envelope are flagged below.

## What changed in v0.3.9 + v0.3.10

The audit re-review reshaped most of the contract's external surface. If the UI was built against the pre-v0.3.9 spec, every contract call needs updating:

| Surface | v0.3.8 form | v0.3.10 form |
|---|---|---|
| `InstantiateMsg` | `{ admin, registry, voting_duration_seconds }` | `{ admin, registry, voting_duration_seconds, registry_update_delay_seconds }` |
| `EnclaveImageRegistry` | `{ mrtd: bytes32, rtmr: bytes32, vkey: string }` | `{ vkey_name, mrtd: bytes48, rtmr1: bytes48, rtmr2: bytes48, rtmr0: bytes48?, rtmr3: bytes48?, accepted_tcb_statuses: u8[] }` |
| `CreateElection` | `{ title, candidates, start_at, end_at, enclave_pubkey }` | `{ title, candidates, start_at, end_at, enclave_pubkey, proof, public_inputs }` — admin must obtain a registration TDX quote from the enclave first |
| `PublishResult` | `{ tally, attestation: DstackAttestation }` | `{ tally, proof, public_inputs }` — no envelope wrapper |
| `UpdateRegistry` | immediate `{ registry }` | **removed**; replaced by `ProposeRegistryUpdate` + `FinalizeRegistryUpdate` + `CancelRegistryUpdate` timelock flow |
| `QueryMsg::PendingRegistry` | did not exist | new — shows `Option<{ registry, apply_after }>` |
| `QueryMsg::HistoricalTally { election_id }` | did not exist | new — archives prior tallies on re-CreateElection |
| `canonical_serialization` preimage | `Borsh(contract_addr) ‖ u64_LE(election_id) ‖ tally_body` | `Borsh(contract_addr) ‖ Borsh(chain_id) ‖ u64_LE(election_id) ‖ ballots_hash[32] ‖ tally_body` (chain_id v0.3.10 N4 cross-chain replay defense; ballots_hash v0.3.11 B6 input fidelity) |

The trust-chain implications are positive: the chain now verifies the gnark Groth16 proof directly via `xion.zk.v1.Query/ProofVerifyGnark`, so the "attestation display" in the UI is genuinely a re-rendering of chain-verified facts — not just a documented future check.

## Trust model the UI must respect

| Computation | Where it MUST run | Why |
|---|---|---|
| ECIES encryption of ballot ranking | Client (browser) | The enclave's public key is published on chain; encrypting client-side is the standard CosmWasm-confidential pattern |
| Attestation verification | Chain-side, read-only display in UI | The chain calls `xion.zk.v1.Query/ProofVerifyGnark` at PublishResult; the UI just renders the extracted measurements + commit hash for human auditability |
| Tally computation | Enclave, off-chain | Never client-side |
| IRV result display | Client (read-only) | Just rendering the on-chain `TallyResult` |

**Anti-pattern to avoid:** computing the tally in JavaScript and displaying it as the result. That bypasses the entire enclave-attestation trust chain. The UI must only display the chain-side `TallyResult` from `query_result()`.

## Tech stack

- **Framework:** Next.js 15+ (App Router) with TypeScript strict mode
- **Wallet / auth:** Xion MetaAccount via Abstraxion (gasless via Treasury) — see `xion-toolkit` skills for setup
- **Contract interaction:** `@burnt-labs/abstraxion` provides signing; `@cosmjs/cosmwasm-stargate` for queries
- **Cryptography:** ECIES (secp256k1) — the encryption MUST match the runtime crate's choice (see `crates/enclave/README.md`). The runtime uses the `ecies` crate (v0.2, secp256k1 + AES-128-GCM). JS-side: `eccrypto-js` matches the wire format; `@noble/curves` + `@noble/ciphers` works if you assemble the ECIES envelope by hand.
- **Styling:** Tailwind (no opinion on component library; `shadcn/ui` is a reasonable default)
- **State:** React Query or SWR for chain reads; URL params for election ID

## Three user flows

### 1. Admin flow

Admin creates and manages elections. Single admin per contract per intent §2.5 Block 1.

```
[Connect wallet] → MetaAccount login
  ↓
[Instantiate contract]                    (one-time per deployment)
  - InstantiateMsg {
      admin,
      registry: EnclaveImageRegistry,        # v0.3.10 schema
      voting_duration_seconds,
      registry_update_delay_seconds,         # NEW v0.3.10 N2: timelock
    }
  - registry_update_delay_seconds: 0 = same-block propose+finalize OK
    (test/dev). Production: ≥86400 (1 day) so voters can react.
  - Returns: contract address
  ↓
[Create election]                         (admin-only, per election)
  - PREREQUISITE: admin must coordinate with the operator running the
    enclave server to obtain a *registration TDX quote* for the
    enclave_pubkey. The runtime's gRPC service is the source of these
    artifacts; see Phase A below.
  - CreateElection {
      title, candidates, start_at, end_at,
      enclave_pubkey,                      # secp256k1 33-byte compressed
      proof,                               # gnark Groth16 proof bytes
      public_inputs,                       # 9_792-byte blob
    }
  - Contract verifies:
      - ProofVerifyGnark passes
      - extracted measurements match registry
      - ReportData[0..32] == SHA-256(enclave_pubkey)
      - ReportData[32..64] == DST_VERIFIED_RCV_PUBKEY_V1 padded
  - Clears prior ballots; archives prior tally (if any) to HISTORICAL_TALLIES.
  - candidates must be distinct + ≥ 2 (Block 1 Requires).
  ↓
[Monitor]                                 (any address can observe)
  - Query Phase → renders Created/Voting/Tallying/Resolved
  - Query Election → metadata + candidates
  - Query Ballots → ballot_count progress
  - Query Result → polled after Tallying; shows TallyResult once resolved
  - Query PendingRegistry → if Some, shows that admin proposed a registry
    rotation and the apply_after time — voters can react
  - Query HistoricalTally { election_id } → archived prior election's tally
  ↓
[Optional: rotate enclave image]          (timelocked, v0.3.10 N2)
  - ProposeRegistryUpdate { registry } → admin-only, stores pending
  - WAIT registry_update_delay_seconds
  - FinalizeRegistryUpdate {} → PERMISSIONLESS (anyone can call after
    timelock expires; prevents admin-abandons-pending soft-brick)
  - Or: CancelRegistryUpdate {} → admin-only, discard pending
  - Only allowed when no election is in Voting/Tallying phase
```

#### Phase A — getting registration / publish artifacts from the enclave

The UI does not produce the `(proof, public_inputs)` pair itself. The admin's flow involves an operational step: call the enclave server's gRPC API and forward the response.

For **registration** quotes (CreateElection):

```
admin UI → operator → enclave server's RegisterPubkey RPC
  (NOT YET IMPLEMENTED — currently admin pre-fetches the pubkey from
   dstack and the operator runs a separate registration tool; future
   work to expose this via gRPC like Tally)
  → returns { enclave_pubkey, proof, public_inputs }
admin UI builds CreateElection msg + signs
```

For **publish** quotes (PublishResult): the enclave runs this end-to-end after `CloseAndTally` fires. The UI typically does NOT call PublishResult directly — the operator's orchestrator does. UI's role is to display the resolved tally + extracted attestation facts.

### 2. Voter (candidate) flow

Only candidates can submit ballots per intent §2.5 Block 3 (S4 + B6 chain-side check). The UI gates the form on `msg.sender ∈ election.candidates`.

```
[Connect wallet] → MetaAccount login
  ↓
[Check eligibility]
  - Query Election → if sender ∉ candidates, show "you are not a candidate"
  - Query Phase → if not Voting, show window state
  - Query Ballots → if sender already submitted, show AlreadyVoted lock
  ↓
[Rank candidates]                         (drag-and-drop or dropdown)
  - Build Vec<Addr> of candidates in preference order
  - Validate: permutation of election.candidates (intent §2.5 Stage 1
    requirement; UI enforces it client-side; enclave re-validates)
  - Show ordering with confirmation step
  ↓
[Encrypt + submit]
  - Borsh-encode the Vec<Addr> ranking
  - ECIES-encrypt under election.enclave_pubkey (queried at instantiate)
  - SubmitBallot { ciphertext: HexBinary }
  - Show tx hash + confirmation
```

**Borsh encoding for ballot plaintext:** Stage 1 decoder parses `Vec<Addr>` using Borsh per intent §2.5. `Addr` is serialized as Borsh `String` (u32-LE length prefix + UTF-8 bytes of bech32). Wire-format mismatch = dropped ballot. The UI's encoding test must round-trip against `crates/enclave/src/bin/roundtrip.rs` (a Rust CLI that takes hex ciphertext + privkey and emits plaintext).

### 3. Observer flow

Anyone can read the chain state. No wallet needed.

```
[Land on election page]
  ↓
[Show phase + window]
  - Created: "voting opens at {start_at}"
  - Voting: "{ballot_count} / {candidate_count} submitted; closes at {end_at}"
  - Tallying: "voting closed; enclave computing tally..."
  - Resolved: "tally published"
  ↓
[On Resolved] show TallyResult
  - Winners, per-round counts, eliminated-by-round, ballots breakdown
  - **Attestation card**: re-rendered from chain-verified facts
    - The on-chain registry's (mrtd, rtmr1, rtmr2) values
    - The on-chain registry's vkey_name (linked to xion.zk store)
    - Note: the chain already verified the gnark proof at PublishResult;
      the UI just shows what the chain checked. No client-side proof
      verification needed for the MVP.
  - Render commit_hash = SHA-256(canonical_serialization(
      contract_addr ‖ chain_id ‖ election_id ‖ tally_body)) and explain
    that this is what the enclave's TDX quote committed to.
  ↓
[If PendingRegistry] show banner
  - "Admin has proposed a new enclave image. New image takes effect
     after {apply_after}. Review at /admin/registry."
  - This is the load-bearing voter-detection window for N2 timelock.
```

## Contract surface to wrap (v0.3.10)

### Query messages

```ts
type Phase = "Created" | "Voting" | "Tallying" | "Resolved";

type Config = {
  admin: string;
  voting_duration_seconds: number;
  registry_update_delay_seconds: number;          // NEW v0.3.10 N2
};

type Election = {
  id: number;
  title: string;
  candidates: string[];
  start_at: { nanos: string };
  end_at: { nanos: string };
  ballot_count: number;
  enclave_pubkey: string;                         // hex; secp256k1
};

type EnclaveImageRegistry = {                     // v0.3.10 schema
  vkey_name: string;                              // refs xion.zk store
  mrtd: string;                                   // base64; 48 bytes
  rtmr1: string;                                  // base64; 48 bytes
  rtmr2: string;                                  // base64; 48 bytes
  rtmr0: string | null;                           // base64; 48 bytes OR null
  rtmr3: string | null;                           // base64; 48 bytes OR null
  accepted_tcb_statuses: number[];                // 0..=5 (6=Revoked rejected)
};

type PendingRegistry = {                          // NEW v0.3.10 N2
  registry: EnclaveImageRegistry;
  apply_after: { nanos: string };
};

type PendingRegistryResponse = {
  pending: PendingRegistry | null;
};

type RoundCount = { candidate: string; count: number };

type TallyResult = {
  winners: string[];
  per_round_counts: RoundCount[][];
  eliminated_by_round: string[][];
  ballots_tallied: number;
  ballots_dropped: number;
  dropped_voters: string[];
  non_voters: string[];
};

type ResultResponse = { result: TallyResult | null };
type BallotsResponse = { ballots: [string, string /* hex */][] };

type QueryMsg =
  | { config: {} }
  | { election: {} }
  | { phase: {} }
  | { ballots: {} }
  | { result: {} }
  | { historical_tally: { election_id: number } }      // NEW v0.3.10 N3
  | { registry: {} }
  | { pending_registry: {} };                          // NEW v0.3.10 N2
```

Wrap each as a typed React Query hook: `useConfig()`, `useElection()`, `usePhase()`, `useBallots()`, `useResult()`, `useHistoricalTally(id)`, `useRegistry()`, `usePendingRegistry()`.

### Execute messages

```ts
type ExecuteMsg =
  | { create_election: {
        title: string;
        candidates: string[];
        start_at: { nanos: string };
        end_at: { nanos: string };
        enclave_pubkey: string;                   // hex
        proof: string;                            // hex; gnark proof
        public_inputs: string;                    // hex; 9_792 bytes
      } }
  | { submit_ballot: { ciphertext: string /* hex */ } }
  | { close_and_tally: {} }
  | { publish_result: {
        tally: TallyResult;
        proof: string;                            // hex; gnark proof
        public_inputs: string;                    // hex; 9_792 bytes
      } }
  | { propose_registry_update: { registry: EnclaveImageRegistry } }   // NEW
  | { finalize_registry_update: {} }                                  // NEW permissionless
  | { cancel_registry_update: {} };                                   // NEW admin
```

`publish_result` is enclave-orchestrator-only in practice; UI does not expose it. `close_and_tally` is permissionless — UI can expose a "trigger tally" button once `block.time ≥ end_at`. `finalize_registry_update` is permissionless and could expose a "complete registry rotation" button to anyone after `apply_after`.

Wrap each as a typed mutation: `useCreateElection()`, `useSubmitBallot()`, `useCloseAndTally()`, `useProposeRegistryUpdate()`, `useFinalizeRegistryUpdate()`, `useCancelRegistryUpdate()`.

## Error shapes from the contract (v0.3.10)

| Error variant | UI handling |
|---|---|
| `Unauthorized` | "Only the admin can do that" |
| `NotEnoughCandidates` | "Election needs ≥ 2 candidates" |
| `DuplicateCandidate` | "Candidate list has duplicates" |
| `InvalidVotingWindow` | "Start must be in the future and before end" |
| `NotVoting` | "Voting window is closed (or not yet open)" |
| `VoterNotCandidate` | "Only candidates can submit ballots" |
| `AlreadyVoted` | "You've already voted (B6 single-write)" |
| `NotTallying` | "Election is not in tallying phase" |
| `AlreadyResolved` | "Tally already published (B1 write-once)" |
| `InvalidEnclavePubkey { got }` | "Bad enclave_pubkey shape (expected 33 or 65 bytes)" |
| `InvalidRegistry(reason)` | "Registry shape invalid: {reason}" |
| `ElectionAlreadyActive` | "An election is in progress; wait for it to resolve" |
| `RegistryUpdateDuringActiveElection` | "Cannot rotate registry while an election is active" |
| `AttestationCommitMismatch` | "Attestation binds to a different tally" |
| `AttestationDomainTagInvalid` | "Attestation domain tag mismatch (cross-purpose replay?)" |
| `GnarkPublicInputsLength { got, expected }` | "Bad public_inputs length: {got} (expected {expected})" |
| `GnarkPublicInputNotU8 { elem_idx }` | "Bad gnark layout at element {elem_idx}" |
| `GnarkPublicInputOutOfRange { elem_idx }` | "Bad gnark value at element {elem_idx}" |
| `ProofVerificationFailed` | "Gnark proof did not verify on chain" |
| `AttestationMeasurementMismatch { field }` | "Enclave measurement {field} ≠ registry" |
| `AttestationPubkeyBindingMismatch` | "Enclave_pubkey not bound to the registration TDX quote" |
| `AttestationTcbStatusUnaccepted { status }` | "TDX TCB status {status} not accepted" |
| `RegistryUpdateAlreadyPending` | "A registry update is already pending; cancel or finalize first" |
| `NoPendingRegistryUpdate` | "Nothing to finalize" |
| `RegistryUpdateTimelockNotExpired` | "Timelock has not expired yet" |

Map ContractError JSON to typed errors via a discriminator. Don't render raw strings.

## What the UI is allowed to assume

- The chain enforces every Block 1-6 precondition; the UI does NOT re-implement that logic. It pre-validates for UX (so the user doesn't burn a tx on `NotVoting`) but the chain is authoritative.
- `phase` is queried (derived per intent v0.3.2 A2); never cached past one block.
- Once `tally_result.is_some()`, the value is immutable (B1 write-once); the UI can cache it freely.
- Once `historical_tally(id)` returns `Some`, it never disappears or changes.
- `pending_registry` may flip null → Some → null at any block; never cache more than a few seconds.

## What the UI must NOT do

- Do not encrypt with anything other than `election.enclave_pubkey`. Wrong key = enclave drops the ballot at Stage 1.
- Do not compute IRV in JavaScript. The user sees only `query_result()`.
- Do not display "Resolved" until `tally_result.is_some()` from chain.
- Do not display "attestation verified" without the chain's `PublishResult` having succeeded — the chain doing the gnark verify IS the verification; client-side gnark would be redundant and easy to get wrong.
- Do not generate or display synthetic `proof` / `public_inputs` bytes. The UI MUST get them from the enclave's gRPC service (or the operator's orchestrator).

## Out of scope for the first cut

- Client-side gnark proof re-verification (defer; chain-side check is authoritative)
- Multi-election history view beyond `HistoricalTally` lookup
- Offline ballot encryption
- i18n
- Mobile-specific UX

## Repo layout (current)

```
verified-rcv/
  ui/                                  # exists at HEAD
    package.json
    next.config.ts
    src/
      app/
        page.tsx                       # observer landing
        admin/page.tsx                 # admin flow
        vote/page.tsx                  # candidate ballot submission
        trust/page.tsx                 # attestation + trust chain breadcrumbs
      components/
        PhaseBadge.tsx
        BallotForm.tsx
        TallyDisplay.tsx
        AttestationCard.tsx
        ContractErrorBanner.tsx
        ConfigBanner.tsx
        EmptyElection.tsx
        Skeleton.tsx
        WalletButton.tsx
        Providers.tsx
      lib/
        contract.ts                    # contract address constant + client
        queries.ts                     # all useQuery wrappers
        mutations.ts                   # all useMutation wrappers
        encryption.ts                  # ECIES + Borsh
        format.ts
        types.ts                       # mirror of msg.rs types — UPDATE per v0.3.10
      hooks/
        useEligibility.ts
    test/
      encryption.test.ts               # shells out to verified-rcv-roundtrip
```

## Pre-implementation checklist

1. **types.ts MUST be regenerated** from the v0.3.10 contract msg.rs. The shapes above are normative.
2. **Enclave runtime's encryption choice is pinned** at `ecies` v0.2 (secp256k1 + AES-128-GCM); `crates/enclave/src/bin/roundtrip.rs` is the round-trip oracle.
3. **The chain endpoint** (Xion testnet vs local CosmWasm). Document in `ui/.env.example`.
4. **The contract address** at deployment. Recipe in `docs/deploy.md`.
5. **vkey_name registration**: confirm with operator that the gnark vkey is registered in Xion's `xion.zk` VKey store under the same `vkey_name` the registry references; until that lands, every PublishResult will fail with `ProofVerificationFailed`. See deploy.md.

## Wiring to the verification trust chain

The UI's "Attestation" view is the user-facing surface of intent §3.2 B10. Make the trust chain INSPECTABLE:

- Show the `EnclaveImageRegistry` `(vkey_name, mrtd, rtmr1, rtmr2, rtmr0?, rtmr3?)` on the election page. This is the image-identity-binding artifact (intent §8.7 step 4).
- On Resolved, show that the chain successfully verified the gnark proof at PublishResult time. The chain's `ProofVerifyGnark` call IS the verification; the UI doesn't need to re-do it.
- Show the `commit_hash = SHA-256(canonical_serialization(contract_addr ‖ chain_id ‖ election_id ‖ tally_body))` and explain it commits to (contract, chain, election, output).
- If `PendingRegistry` is Some, show the apply_after time prominently. This is the v0.3.10 N2 voter-detection window.
- Link to `.colosseum/intent.md` and `.colosseum/ledger.md` so a sufficiently-motivated observer can audit the full trust chain.

A user who clicks "why should I trust this result?" should be able to walk from the UI → intent → ledger → Lean proofs. Make the breadcrumbs explicit.

## Known limitations the UI should disclose honestly

1. **~~`enclave_input_fidelity` is still open~~ DISCHARGED v0.3.11 B6** (intent §8.7 link 7). The chain now binds BOTH the *output* (tally) and the *input* (`ballots_hash`) into the publish-quote commit. A host that substitutes ballots produces a different `ballots_hash`; the chain rejects with `AttestationCommitMismatch`. UI's Trust page can show this as "input binding: ✓" alongside the existing output-binding facts.
2. **`B10_lean` has a `sorry`**. The Lean theorem that "EnclaveImage = Tally_spec" is incomplete. The math correctness obligation is documented future work. Surface this on the Trust page.
3. **Admin trust surface remains for VM-with-matching-MRTD**. After N2 timelock, an admin who controls hardware with the right MRTD and accepts the visibility cost of the timelock can still substitute a different VM. Disclose under "trust assumptions."

## Open questions to resolve before implementation

1. The `RegisterPubkey` gRPC RPC for the enclave server does not exist yet — admin currently obtains the registration `(proof, public_inputs)` via a side-channel coordination with the operator. Decide whether to add this RPC or keep the side-channel.
2. Should the UI surface a "stale ballot warning" if the user submits an old ranking after `PendingRegistry` has been proposed? (Their ballot remains valid; the pending registry only affects future elections.)
3. `block.chain_id` is read on chain at PublishResult time; the UI must use the same `chain_id` when computing client-side commit hashes for display. Confirm the chain_id source.
