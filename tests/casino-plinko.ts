import * as anchor from '@project-serum/anchor';
import { Program, Idl } from '@project-serum/anchor';
import { CasinoPlinko } from '../target/types/casino_plinko';
import { expect } from 'chai';
import * as path from 'path';
import * as fs from 'fs';

// Load the Solana wallet keypair
const walletPath = path.join(process.env.HOME || require('os').homedir(), '.config', 'solana', 'id.json');
const walletKeypair = anchor.web3.Keypair.fromSecretKey(
  new Uint8Array(JSON.parse(fs.readFileSync(walletPath, 'utf-8')))
);

// Set the provider URL for Devnet
const provider = new anchor.AnchorProvider(
  new anchor.web3.Connection("https://api.devnet.solana.com"),
  new anchor.Wallet(walletKeypair),
  {}
);
anchor.setProvider(provider);

// Load the IDL and program ID
const idl = JSON.parse(fs.readFileSync('./target/idl/casino_plinko.json', 'utf-8'));
const programId = new anchor.web3.PublicKey("7CNPz8SgNAp2GhyJvnmBs2qGwzeJ8NBQDnfDbT7juR9p");

// Create the program instance
const program = new Program<CasinoPlinko>(idl, programId, provider);

describe('casino_plinko', () => {
  it('Initializes player account and places a bet', async () => {
    // Generate a new keypair for the player account
    const playerAccount = anchor.web3.Keypair.generate();

    // Initialize player account
    await program.methods.initializePlayer(new anchor.BN(100))
      .accounts({
        playerAccount: playerAccount.publicKey,
        player: provider.wallet.publicKey,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .signers([playerAccount])
      .rpc();

    // Fetch the initialized account
    let account = await program.account.playerAccount.fetch(playerAccount.publicKey);
    expect(account.balance.toString()).to.equal('100');

    // Place a bet
    const gameAccount = anchor.web3.Keypair.generate();
    await program.methods.placeBet(new anchor.BN(50))
      .accounts({
        playerAccount: playerAccount.publicKey,
        gameAccount: gameAccount.publicKey,
        player: provider.wallet.publicKey,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .signers([gameAccount])
      .rpc();

    // Fetch the game account
    let game = await program.account.gameAccount.fetch(gameAccount.publicKey);
    expect(game.betAmount.toString()).to.equal('50');
    expect(game.result).to.equal(0);

    // Determine the result
    await program.methods.determineResult(1)
      .accounts({
        gameAccount: gameAccount.publicKey,
        playerAccount: playerAccount.publicKey,
        player: provider.wallet.publicKey,
      })
      .rpc();

    // Fetch the updated player account
    account = await program.account.playerAccount.fetch(playerAccount.publicKey);
    expect(account.balance.toString()).to.equal('150');
  });
});