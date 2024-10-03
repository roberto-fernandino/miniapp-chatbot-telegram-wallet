import { useState, useEffect } from "react";
import { getAllSolanaTokensBalance } from "../../lib/utils";
import { PublicKey, AccountInfo, ParsedAccountData } from "@solana/web3.js";
import { Spinner } from "./spinner";
import Token from "./token";
interface TokenInfo {
  pubkey: PublicKey;
  account: AccountInfo<ParsedAccountData>;
}

const TokensBalances: React.FC<{ address: string }> = ({ address }) => {
  const [tokens, setTokens] = useState<TokenInfo[]>([]);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    setLoading(true);
    getAllSolanaTokensBalance(address).then((response) => {
      setTokens(response.value);
      setLoading(false);
      let count = 0;
      tokens.map((token: TokenInfo) => {
        if (token.account.data.parsed.info.tokenAmount.uiAmount > 0) {
          count++;
        }
      });
    });
  }, [address]);

  return (
    <div className="flex flex-col items-center justify-center w-full mt-3">
      {loading ? (
        <Spinner />
      ) : (
        <>
          <h2 className="text-2xl font-bold bg-gradient-to-r from-purple-500 to-pink-500 text-transparent bg-clip-text mb-2">
            Token Portfolio
          </h2>
          <span className="text-[8px] mt-[-5px] text-gray-500 mb-2"></span>
          {tokens.map((token: TokenInfo) => (
            <Token key={token.pubkey.toBase58()} token={token} />
          ))}
        </>
      )}
    </div>
  );
};

export default TokensBalances;
