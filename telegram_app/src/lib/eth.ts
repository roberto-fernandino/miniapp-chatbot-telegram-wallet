import { isAddress } from "ethers";

export function isValidEthereumAddress(address: string): boolean {
  return isAddress(address);
}

/**
 * Get the price of ETH.
 * @returns {Promise<number>} The price of ETH.
 */
export async function getETHPrice(): Promise<number> {
  const response = await fetch(
    "https://api.coingecko.com/api/v3/simple/price?ids=ethereum&vs_currencies=usd"
  );
  const data = await response.json();
  return data.ethereum.usd;
}
