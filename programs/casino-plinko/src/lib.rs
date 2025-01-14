use anchor_lang::prelude::*;

// Declare the program ID
declare_id!("Bfdzd1XoVrsmuj93dBpi6s4AMgJMmKX2fMfYUUKwCr77");

// Constants
const MAX_INITIAL_BALANCE: u64 = 1_000_000; // Example max initial balance
const PLAYER_ACCOUNT_SPACE: usize = 8 + 8; // 8 (discriminator) + 8 (balance)
const GAME_ACCOUNT_SPACE: usize = 8 + 8 + 8 + 1; // 8 (discriminator) + 8 (balance) + 8 (bet amount) + 1 (result)
const GAME_ADMIN: Pubkey = pubkey!("6HYF3mjwcFADoBh55FsTNFSxC4Gos5yU6AsATTQ64oHW"); // Replace with actual admin pubkey

#[program]
pub mod casino_plinko {
    use super::*;

    /// Initialize the player account
    pub fn initialize_player(ctx: Context<InitializePlayer>, initial_balance: u64) -> Result<()> {
        require!(initial_balance > 0, PlinkoBetError::InvalidInitialBalance);
        require!(
            initial_balance <= MAX_INITIAL_BALANCE,
            PlinkoBetError::InvalidInitialBalance
        );

        let player_account = &mut ctx.accounts.player_account;
        player_account.balance = initial_balance;

        emit!(PlayerInitialized {
            player: ctx.accounts.player.key(),
            initial_balance,
            timestamp: Clock::get()?.unix_timestamp,
        });

        msg!("Player Account Initialized");
        msg!("Player: {}", ctx.accounts.player.key());
        msg!("Initial Balance: {}", initial_balance);

        Ok(())
    }

    /// Initialize the game account
    pub fn initialize_game(ctx: Context<InitializeGame>, initial_balance: u64) -> Result<()> {
        let game_account = &mut ctx.accounts.game_account;

        game_account.balance = initial_balance;
        game_account.bet_amount = 0; // No bet yet
        game_account.result = GameResult::Pending; // No result yet

        emit!(GameInitialized {
            game: ctx.accounts.game_account.key(),
            initial_balance,
            timestamp: Clock::get()?.unix_timestamp,
        });

        msg!("Game Account Initialized");
        msg!("Initial Game Balance: {}", initial_balance);

        Ok(())
    }

    /// Place a bet
    pub fn place_bet(ctx: Context<PlaceBet>, bet_amount: u64) -> Result<()> {
        require!(bet_amount > 0, PlinkoBetError::InvalidBetAmount);

        let player_account = &mut ctx.accounts.player_account;

        require!(
            player_account.balance >= bet_amount,
            PlinkoBetError::InsufficientBalance
        );
        player_account.balance = player_account
            .balance
            .checked_sub(bet_amount)
            .ok_or(PlinkoBetError::Underflow)?;

        let game_account = &mut ctx.accounts.game_account;
        game_account.bet_amount = bet_amount;
        game_account.result = GameResult::Pending;

        emit!(BetPlaced {
            player: ctx.accounts.player.key(),
            bet_amount,
            timestamp: Clock::get()?.unix_timestamp,
        });

        msg!("Bet placed successfully by {}", ctx.accounts.player.key());
        msg!("Bet Amount: {}", bet_amount);
        msg!("Updated Player Balance: {}", player_account.balance);

        Ok(())
    }

    /// Determine the result of the game
   /// Determine the result of the game with a multiplier
pub fn determine_result(
    ctx: Context<DetermineResult>,
    result: GameResult,
    multiplier: u64, // New parameter for the multiplier
) -> Result<()> {
    require!(
        ctx.accounts.player.key() == GAME_ADMIN,
        PlinkoBetError::Unauthorized
    );

    let game_account = &mut ctx.accounts.game_account;
    let player_account = &mut ctx.accounts.player_account;

    game_account.result = result;

    if let GameResult::Win = result {
        let winnings = game_account
            .bet_amount
            .checked_mul(multiplier) // Multiply by the provided multiplier
            .ok_or(PlinkoBetError::Overflow)?;
        player_account.balance = player_account
            .balance
            .checked_add(winnings)
            .ok_or(PlinkoBetError::Overflow)?;
    }

    emit!(ResultDetermined {
        player: ctx.accounts.player.key(),
        result: game_account.result,
        winnings: if let GameResult::Win = result {
            game_account.bet_amount * multiplier
        } else {
            0
        },
        timestamp: Clock::get()?.unix_timestamp,
    });

    msg!("Game result determined for player {}", ctx.accounts.player.key());
    msg!("Result: {:?}", game_account.result);
    msg!("Updated Player Balance: {}", player_account.balance);

    Ok(())
}
    /// Deposit funds into the player's account
    pub fn deposit_funds(ctx: Context<DepositFunds>, amount: u64) -> Result<()> {
        require!(amount > 0, PlinkoBetError::InvalidDepositAmount);

        let player_account = &mut ctx.accounts.player_account;
        let player = &mut ctx.accounts.player;

        // Transfer funds from the player's wallet to the player account
        **player.to_account_info().try_borrow_mut_lamports()? -= amount;
        **player_account.to_account_info().try_borrow_mut_lamports()? += amount;

        // Update the player account balance
        player_account.balance = player_account
            .balance
            .checked_add(amount)
            .ok_or(PlinkoBetError::Overflow)?;

        emit!(FundsDeposited {
            player: ctx.accounts.player.key(),
            amount,
            timestamp: Clock::get()?.unix_timestamp,
        });

        msg!("Deposit successful for {}", ctx.accounts.player.key());
        msg!("Deposit Amount: {}", amount);
        msg!("Updated Player Balance: {}", player_account.balance);

        Ok(())
    }

    /// Close the player account and reclaim rent-exempt SOL
    pub fn close_player_account(ctx: Context<ClosePlayerAccount>) -> Result<()> {
        let player_account = &mut ctx.accounts.player_account;
        let player = &mut ctx.accounts.player;

        // Transfer remaining balance to the player
        **player.to_account_info().try_borrow_mut_lamports()? += player_account.balance;
        **player_account.to_account_info().try_borrow_mut_lamports()? = 0;

        emit!(PlayerAccountClosed {
            player: ctx.accounts.player.key(),
            timestamp: Clock::get()?.unix_timestamp,
        });

        msg!("Player account closed for {}", ctx.accounts.player.key());

        Ok(())
    }
}

#[derive(Accounts)]
pub struct InitializePlayer<'info> {
    #[account(
        init,
        payer = player,
        space = PLAYER_ACCOUNT_SPACE,
        seeds = [b"player_account", player.key().as_ref()],
        bump
    )]
    pub player_account: Account<'info, PlayerAccount>,
    #[account(mut)]
    pub player: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct InitializeGame<'info> {
    #[account(
        init,
        payer = player,
        space = GAME_ACCOUNT_SPACE,
        seeds = [b"game_account", player.key().as_ref()],
        bump
    )]
    pub game_account: Account<'info, GameAccount>,
    #[account(mut)]
    pub player: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct PlaceBet<'info> {
    #[account(
        mut,
        seeds = [b"player_account", player.key().as_ref()],
        bump
    )]
    pub player_account: Account<'info, PlayerAccount>,
    #[account(
        mut,
        seeds = [b"game_account", player.key().as_ref()],
        bump
    )]
    pub game_account: Account<'info, GameAccount>,
    #[account(mut)]
    pub player: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct DetermineResult<'info> {
    #[account(
        mut,
        seeds = [b"game_account", player.key().as_ref()],
        bump
    )]
    pub game_account: Account<'info, GameAccount>,
    #[account(
        mut,
        seeds = [b"player_account", player.key().as_ref()],
        bump
    )]
    pub player_account: Account<'info, PlayerAccount>,
    pub player: Signer<'info>,
}

#[derive(Accounts)]
pub struct DepositFunds<'info> {
    #[account(mut)]
    pub player_account: Account<'info, PlayerAccount>,
    #[account(mut)]
    pub player: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct ClosePlayerAccount<'info> {
    #[account(
        mut,
        seeds = [b"player_account", player.key().as_ref()],
        bump,
        close = player
    )]
    pub player_account: Account<'info, PlayerAccount>,
    #[account(mut)]
    pub player: Signer<'info>,
}

#[account]
pub struct PlayerAccount {
    pub balance: u64,
}

#[account]
pub struct GameAccount {
    pub balance: u64,
    pub bet_amount: u64,
    pub result: GameResult,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, Debug)]
pub enum GameResult {
    Pending,
    Win,
    Loss,
}

#[event]
pub struct PlayerInitialized {
    pub player: Pubkey,
    pub initial_balance: u64,
    pub timestamp: i64,
}

#[event]
pub struct GameInitialized {
    pub game: Pubkey,
    pub initial_balance: u64,
    pub timestamp: i64,
}

#[event]
pub struct BetPlaced {
    pub player: Pubkey,
    pub bet_amount: u64,
    pub timestamp: i64,
}

#[event]
pub struct ResultDetermined {
    pub player: Pubkey,
    pub result: GameResult,
    pub winnings: u64, // This will now include the multiplier
    pub timestamp: i64,
}

#[event]
pub struct FundsDeposited {
    pub player: Pubkey,
    pub amount: u64,
    pub timestamp: i64,
}

#[event]
pub struct PlayerAccountClosed {
    pub player: Pubkey,
    pub timestamp: i64,
}

#[error_code]
pub enum PlinkoBetError {
    #[msg("Insufficient balance")]
    InsufficientBalance,
    #[msg("Invalid initial balance")]
    InvalidInitialBalance,
    #[msg("Invalid bet amount")]
    InvalidBetAmount,
    #[msg("Unauthorized access")]
    Unauthorized,
    #[msg("Arithmetic overflow")]
    Overflow,
    #[msg("Arithmetic underflow")]
    Underflow,
    #[msg("Account already initialized")]
    AlreadyInitialized,
    #[msg("Invalid deposit amount")]
    InvalidDepositAmount,
}