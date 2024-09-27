import { Connection, PublicKey } from "@solana/web3.js";


export async function getWalletSolBalance(walletAddress:string) {

    const connection = new Connection(import.meta.env.VITE_RPC_URL!, "confirmed");


    try {
        const publicKey = new PublicKey(walletAddress);

        const balance = await connection.getBalance(publicKey);

        const solBalance = balance / 10 ** 9;
        const usdtBalance = solBalance * await getSolPrice();

        return {solBalance, usdtBalance};
    } catch (error) {
        console.error("Error fetching SOL balance:", error);
        throw error;
    }
}

async function getSolPrice() {
    const response = await fetch("https://api.coingecko.com/api/v3/simple/price?ids=solana&vs_currencies=usd");
    const data = await response.json();
    return data.solana.usd;
}

