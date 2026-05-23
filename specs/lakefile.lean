import Lake
open Lake DSL

package «verified-rcv» where
  leanOptions := #[
    ⟨`pp.unicode.fun, true⟩,
    ⟨`autoImplicit, false⟩
  ]

require mathlib from git
  "https://github.com/leanprover-community/mathlib4.git" @ "v4.29.0"

require «VCV-io» from git
  "https://github.com/Verified-zkEVM/VCV-io.git" @ "v4.29.0"

@[default_target]
lean_lib RcvSpec where
  -- single-file library; the file is RcvSpec.lean at package root
