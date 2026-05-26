# verified-rcv UI integration spec

This document specifies the frontend integration for verified-rcv. The UI is **orthogonal to the verification trust chain** — it presents and orchestrates, but contributes no trust-bearing computation. The trust chain ends at chain state + enclave attestation; the UI just renders both.

Spin up the UI in its own session. This document is the brief.

## Trust model the UI must respect

| Computation | Where it MUST run | Why |
|---|---|---|
| ECIES encryption of ballot ranking | Client (browser) | The enclave's public key is published on chain; encrypting client-side is the standard CosmWasm-confidential pattern |
| Attestation verification | Read-only display | The chain does the load-bearing check at `PublishResult`; the UI just shows the envelope contents |
| Tally computation | Enclave, off-chain | Never client-side |
| IRV result display | Client (read-only) | Just rendering the on-chain `TallyResult` |

**Anti-pattern to avoid:** computing the tally in JavaScript and displaying it as the result. That bypasses the entire enclave-attestation trust chain. The UI must only display the chain-side `TallyResult` from `query_result()`.

## Tech stack

- **Framework:** Next.js 15+ (App Router) with TypeScript strict mode
- **Wallet / auth:** Xion MetaAccount via Abstraxion (gasless via Treasury) — see `xion-toolkit` skills for setup
- **Contract interaction:** `@burnt-labs/abstraxion` provides signing; `@cosmjs/cosmwasm-stargate` for queries
- **Cryptography:** ECIES (secp256k1) via `@noble/curves` + `@noble/ciphers` for ChaCha20-Poly1305, OR `eccrypto-js` if AES-GCM matches the enclave runtime crate's choice
- **Styling:** Tailwind (no opinion on component library; `shadcn/ui` is a reasonable default)
- **State:** React Query or SWR for chain reads; URL params for election ID

**Important:** the cryptography choice must match what the enclave runtime crate uses. The runtime crate is currently a stub. The UI and the runtime crate must agree on the encryption scheme — pin it in the runtime crate first, then mirror in the UI. Document the choice in `crates/enclave/README.md` once written.

## Three user flows

### 1. Admin flow

Admin creates and monitors elections. Single admin per contract per intent §2.5 Block 1.

```
[Connect wallet] → MetaAccount login
  ↓
[Instantiate contract]               (one-time per deployment)
  - InstantiateMsg { admin, registry, voting_duration_seconds }
  - Registry is the (mrtd, rtmr, vkey) tuple for the expected enclave image
  - Returns: contract address
  ↓
[Create election]                    (admin-only, per election)
  - CreateElection { title, candidates, start_at, end_at }
  - Clears prior ballots + tally_result
  - candidates is Vec<Addr>, must be distinct + non-empty (Block 1 Requires)
  - start_at > now; end_at > start_at
  ↓
[Monitor]                            (any address can observe)
  - Query Phase → renders Created/Voting/Tallying/Resolved
  - Query Election → renders election metadata + candidate list
  - Query Ballots → shows ballot_count progress; entries are ciphertext-opaque
  - Query Result → polled after Tallying; shows TallyResult once resolved
```

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
[Rank candidates]                    (drag-and-drop or dropdown)
  - Build Vec<Addr> of candidates in preference order
  - Validate: permutation of election.candidates (intent §2.5 Stage 1
    requirement; UI enforces it client-side; enclave re-validates)
  - Show the ordering clearly with confirmation step
  ↓
[Encrypt + submit]
  - Borsh-encode the Vec<Addr> ranking (matches Stage 1 decoder)
  - ECIES-encrypt under election.enclave_pubkey (queried at instantiate)
  - SubmitBallot { ciphertext: HexBinary }
  - Show tx hash + confirmation
```

**Borsh encoding for ballot plaintext:** the Stage 1 decoder in the runtime crate parses `Vec<Addr>` using Borsh per intent §2.5 canonical_serialization pin. `Addr` is serialized as Borsh `String` (u32-LE length prefix + UTF-8 bytes of bech32). Wire-format mismatch here means the enclave drops the ballot. The UI MUST use the exact same encoding — pin a JS Borsh library and write a unit test against a known plaintext.

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
  - Winners list
  - Per-round counts (table with candidate × round)
  - Eliminated-by-round (parallel list)
  - ballots_tallied / ballots_dropped / non_voters breakdown
  - **Attestation envelope display** — show that the result was attested:
    - If Mock: render "DEV BUILD: mock attestation" warning prominently
    - If Dstack: render the quote/zk_proof/user_data shapes; provide a
      "verify locally" button that re-runs the chain-side checks (B8 clause
      (c) hash equality; B8 (a)+(b) require zkdcap verifier in JS — out of
      scope for the first cut, document as future work)
```

## Contract surface to wrap

### Query messages

```ts
type Phase = "Created" | "Voting" | "Tallying" | "Resolved";

type Config = {
  admin: string;
  voting_duration_seconds: number;
};

type Election = {
  id: number;
  title: string;
  candidates: string[];
  start_at: { nanos: string };
  end_at: { nanos: string };
  ballot_count: number;
};

type EnclaveImageRegistry = {
  mrtd: string;     // base64 32 bytes
  rtmr: string;     // base64 32 bytes
  vkey: string;     // string identifier
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
```

Wrap each as a typed React Query hook: `useConfig()`, `useElection()`, `usePhase()`, `useBallots()`, `useResult()`, `useRegistry()`.

### Execute messages

```ts
type ExecuteMsg =
  | { create_election: { title: string; candidates: string[]; start_at: { nanos: string }; end_at: { nanos: string } } }
  | { submit_ballot: { ciphertext: string /* hex */ } }
  | { close_and_tally: {} }
  | { publish_result: { tally: TallyResult; attestation: AttestationEnvelope } };
```

`publish_result` is enclave-only in practice; UI does not expose it. `close_and_tally` could expose a "trigger tally" button for observers once `block.time ≥ end_at` (it's permissionless per intent §2.5 Block 5).

Wrap each as a typed mutation: `useCreateElection()`, `useSubmitBallot()`, `useCloseAndTally()`.

## Error shapes from the contract

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
| `AttestationFailure(reason)` | "Tally validation failed: {reason}" (S6-S9 violations) |

Map ContractError JSON to typed errors via a discriminator. Don't render raw strings.

## What the UI is allowed to assume

- The chain enforces every Block 1-6 precondition; the UI does NOT re-implement that logic. It pre-validates for UX (so the user doesn't burn a tx on `NotVoting`) but the chain is authoritative.
- `phase` is queried (derived per intent v0.3.2 A2); never cached past one block.
- Once `tally_result.is_some()`, the value is immutable (B1 write-once); the UI can cache it freely.

## What the UI must NOT do

- Do not encrypt with anything other than `election.enclave_pubkey`. Wrong key = enclave drops the ballot at Stage 1.
- Do not compute IRV in JavaScript. The user sees only `query_result()`.
- Do not display "Resolved" until `tally_result.is_some()` from chain.
- Do not show the attestation envelope as "verified" without doing the chain-side checks (or just deferring to the chain's success).

## Out of scope for the first cut

- zkdcap proof verification in JS (defer; show envelope contents only)
- Multi-election history view (the contract supports successive elections via `ELECTION_COUNTER` but the UI MVP can pin to the current one)
- Offline ballot encryption (the UI assumes the browser has the enclave pubkey at submit time)
- i18n
- Mobile-specific UX

## Repo layout suggestion

```
verified-rcv/
  ui/                                  # new top-level dir
    package.json
    next.config.ts
    src/
      app/                             # Next.js App Router
        page.tsx                       # observer landing
        admin/page.tsx                 # admin flow
        vote/page.tsx                  # candidate ballot submission
      components/
        PhaseBadge.tsx
        BallotForm.tsx                 # drag-and-drop ranking
        TallyDisplay.tsx               # rounds + winners
        AttestationCard.tsx
      lib/
        contract.ts                    # contract address constant + chain client
        queries.ts                     # all useQuery wrappers
        mutations.ts                   # all useMutation wrappers
        encryption.ts                  # ECIES wrapper, Borsh encoder
        types.ts                       # mirror of msg.rs types
      hooks/
        useEligibility.ts              # combines Election + Phase + Ballots
    test/
      encryption.test.ts               # roundtrip vs a known plaintext (must
                                       # match the runtime crate's decoder)
```

## Pre-implementation checklist

Before writing any UI code, lock down:

1. **Enclave runtime crate's encryption choice** (currently stub). The UI cannot encrypt without knowing the exact ECIES variant (curve, KDF, AEAD). Pin in `crates/enclave/README.md`.
2. **A way to test the encryption roundtrip without the real enclave**. Provide a Rust-side test binary that takes a hex ciphertext + privkey and outputs the decrypted plaintext. The UI test suite calls into it (or a wasm-compiled equivalent) to validate the JS-side encoder.
3. **The chain endpoint** (Xion testnet vs local CosmWasm). Document in `ui/.env.example`.
4. **The contract address** at deployment. Add a `deploy.md` recipe under `docs/`.

## Wiring to the verification trust chain

The UI's "Attestation" view is the user-facing surface of intent §3.2 B10. Even though the chain enforces B10 syntactically, the UI should make the trust chain INSPECTABLE:

- Show the `EnclaveImageRegistry` `(mrtd, rtmr, vkey)` on the election page. This is the image-identity-binding artifact (intent §8.7 step 4).
- On Resolved, show the attestation envelope's `user_data` hash and explain it commits to `(contract_addr, tally_body)`.
- Link to `.colosseum/intent.md` and `.colosseum/ledger.md` so a sufficiently-motivated observer can audit the full trust chain.

A user who clicks "why should I trust this result?" should be able to walk from the UI → intent → ledger → Lean proofs. Make the breadcrumbs explicit.

## Open questions to resolve before implementation

1. Does the Xion MetaAccount Treasury setup accept submitting `SubmitBallot` as a gasless tx? (Likely yes — that's the design — but verify via the `xion-oauth2-client` skill setup.)
2. Is there an existing Burnt Labs starter template for CosmWasm + Abstraxion that should be the base? Check `xion-toolkit-init` for the canonical scaffolding.
3. Does the UI need to support contract migration (CosmWasm `migrate` entry point)? The verified-rcv contract doesn't expose one currently — confirm or add.
