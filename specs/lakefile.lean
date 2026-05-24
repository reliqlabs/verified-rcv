import Lake
open Lake DSL

package «verified-rcv» where
  leanOptions := #[
    ⟨`pp.unicode.fun, true⟩,
    ⟨`autoImplicit, false⟩
  ]

require mathlib from git
  "https://github.com/leanprover-community/mathlib4.git" @ "v4.30.0-rc2"

-- Aeneas as a Lake dep so we can `import Aeneas` from the extracted enclave
-- core (see ../lean-extraction/Enclave-core.lean). Tracks the same Lean
-- toolchain (4.30.0-rc2). The Aeneas backends include `Aeneas.Std` (Slice,
-- Vec, Result monad, etc.) that the extracted file uses.
require aeneas from git
  "https://github.com/AeneasVerif/aeneas.git" / "backends" / "lean"

-- VCV-io is temporarily dropped — no v4.30.0 release yet. Will re-add when
-- VCV-io tracks Lean 4.30. Not currently load-bearing (no crypto proofs yet).

@[default_target]
lean_lib RcvSpec where
  -- single-file library; the file is RcvSpec.lean at package root

-- The extracted Aeneas Lean module from `crates/enclave-core/`. The file
-- was produced by `lake exe aeneas extract` (or equivalent via
-- mcp__aeneas__extract_rust_to_lean) into `../lean-extraction/` and is
-- copied here so Lake can build it as a sibling of RcvSpec. Re-extract
-- when the Rust crate changes; the copy ensures `import EnclaveExtracted`
-- works from RcvSpec.
lean_lib EnclaveExtracted where
  -- file is `EnclaveExtracted.lean` at package root

-- Bridge module: refinement lemmas connecting the extracted enclave core
-- to the math spec. Stages B10_lean_irv (Stage 2) + B10_lean_decrypt
-- (Stage 1) proofs that anchor B10_lean.
lean_lib EnclaveBridge where
  -- file is `EnclaveBridge.lean` at package root
