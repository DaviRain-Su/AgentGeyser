use anchor_lang::prelude::*;

declare_id!("11111111111111111111111111111112");

#[program]
pub mod hello_world {
    use super::*;

    pub fn greet(ctx: Context<Greet>, name: String) -> Result<()> {
        let _ = ctx;
        msg!("hello, {}", name);
        Ok(())
    }
}

#[derive(Accounts)]
pub struct Greet<'info> {
    #[account(mut)]
    pub user: Signer<'info>,
}
