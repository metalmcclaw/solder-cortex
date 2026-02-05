use anchor_lang::prelude::*;

declare_id!("zcAKcWaZFoueQ4XB7AYoqNjVcu6WtLi7dx5kQ7mCsbJ");

#[program]
pub mod solder_dao {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        msg!("Greetings from: {:?}", ctx.program_id);
        Ok(())
    }
}

#[derive(Accounts)]
pub struct Initialize {}
