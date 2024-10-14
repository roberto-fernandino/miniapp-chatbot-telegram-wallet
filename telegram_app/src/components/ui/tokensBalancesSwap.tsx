import { useState, useEffect } from "react";
import { getAllSolanaTokensBalance, getTokenData } from "../../lib/utils";
import { PublicKey, AccountInfo, ParsedAccountData } from "@solana/web3.js";
import { Spinner } from "./spinner";
import SolToken from "./solToken";

interface TokenInfo {
  pubkey: PublicKey;
  account: AccountInfo<ParsedAccountData>;
}
interface TokensBalancesSwapProps {
  address: string;
  setTokenData: (token: any) => void;
  setTokenCa: (token: string) => void;
}
const TokensBalancesSwap: React.FC<TokensBalancesSwapProps> = ({
  address,
  setTokenData,
  setTokenCa,
}) => {
  const [tokens, setTokens] = useState<TokenInfo[]>([]);
  const [loading, setLoading] = useState(true);

  const handleGetTokenData = async (tokenCa: string) => {
    const data = await getTokenData(tokenCa);
    setTokenData(data);
    setTokenCa(tokenCa);
  };

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
          <span className="text-[8px] mt-[-5px] text-gray-500 mb-2"></span>
          {tokens.map((token: TokenInfo) =>
            token.account.data.parsed.info.tokenAmount.uiAmount > 0 ? (
              <div
                className="flex flex-col items-center justify-center w-full mt-2 bg-[#fbcff9] p-3 hover:cursor-pointer rounded-full "
                onClick={() =>
                  handleGetTokenData(token.account.data.parsed.info.mint)
                }
              >
                <SolToken key={token.pubkey.toBase58()} token={token} />
              </div>
            ) : null
          )}
        </>
      )}
    </div>
  );
};

export default TokensBalancesSwap;
