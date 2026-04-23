//! Hand-crafted "native program" skills that are not derived from an Anchor
//! IDL. These are registered directly on the `IdlRegistry` at proxy startup
//! so they are always available (Track B of MVP-M2).
//!
//! The non-custodial invariant still holds: this module never signs anything
//! and holds no key material. It only seeds a deterministic `Skill` entry.

use serde_json::json;
use skill_synth::{IdlType, Skill, SkillAccountSpec, SkillArgSpec};

use crate::IdlRegistry;

/// SPL-Token program ID (Tokenkeg...).
pub const SPL_TOKEN_PROGRAM_ID: &str = "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA";

/// Canonical skill id for the SPL-Token Transfer demo.
pub const SPL_TOKEN_TRANSFER_SKILL_ID: &str = "spl-token::transfer";

/// Register the SPL-Token `Transfer` instruction as a skill on the given
/// registry. This is a native-program path: `discriminator[0]` is the 1-byte
/// instruction tag (`3` for Transfer); the remaining 7 bytes are zero-padded
/// so the field still fits `Skill`'s fixed `[u8; 8]`. The proxy's native
/// dispatch (`handle_invoke` → `build_native_unsigned_tx`) reads only the
/// leading tag and ignores the padding.
pub fn register_spl_token_transfer_skill(registry: &mut IdlRegistry) {
    let skill = Skill {
        skill_id: SPL_TOKEN_TRANSFER_SKILL_ID.to_string(),
        program_id: SPL_TOKEN_PROGRAM_ID.to_string(),
        program_name: Some("spl-token".to_string()),
        instruction_name: "transfer".to_string(),
        params_schema: json!({
            "$schema": "https://json-schema.org/draft/2020-12/schema",
            "title": "transfer",
            "type": "object",
            "properties": {
                "amount": { "type": "integer", "minimum": 0 }
            },
            "required": ["amount"],
            "additionalProperties": false
        }),
        discriminator: [3, 0, 0, 0, 0, 0, 0, 0],
        accounts: vec![
            SkillAccountSpec { name: "source".into(), is_mut: true, is_signer: false },
            SkillAccountSpec { name: "destination".into(), is_mut: true, is_signer: false },
            SkillAccountSpec { name: "authority".into(), is_mut: false, is_signer: true },
        ],
        args: vec![SkillArgSpec { name: "amount".into(), ty: IdlType::U64 }],
    };
    registry.skills.insert(skill.skill_id.clone(), skill);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spl_token_transfer_skill_is_registered() {
        let mut registry = IdlRegistry::default();
        register_spl_token_transfer_skill(&mut registry);
        let skills = registry.list_skills();
        assert!(
            skills.iter().any(|s| s.skill_id == SPL_TOKEN_TRANSFER_SKILL_ID),
            "list_skills must contain spl-token::transfer, got {:?}",
            skills.iter().map(|s| &s.skill_id).collect::<Vec<_>>()
        );
        let s = skills
            .iter()
            .find(|s| s.skill_id == SPL_TOKEN_TRANSFER_SKILL_ID)
            .unwrap();
        assert_eq!(s.program_id, SPL_TOKEN_PROGRAM_ID);
        assert_eq!(s.instruction_name, "transfer");
        assert_eq!(s.discriminator, [3, 0, 0, 0, 0, 0, 0, 0]);
        assert_eq!(s.accounts.len(), 3);
        assert!(s.accounts[0].is_mut && !s.accounts[0].is_signer);
        assert!(s.accounts[1].is_mut && !s.accounts[1].is_signer);
        assert!(!s.accounts[2].is_mut && s.accounts[2].is_signer);
        assert_eq!(s.args.len(), 1);
        assert_eq!(s.args[0].name, "amount");
        assert_eq!(s.args[0].ty, IdlType::U64);
    }
}
