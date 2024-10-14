import { useState, useEffect } from "react";
import { getAllEthereumTokensBalance } from "../../lib/utils";
import { Spinner } from "./spinner";
import EthToken from "./ethToken";
interface TokenInfo {
  token_address: string;
  name: string;
  symbol: string;
  logo?: string;
  thumbnail?: string;
  decimals: number;
  balance: string;
  possible_spam: boolean;
  verified_contract?: boolean;
}

const EthTokenBalances: React.FC<{ address: string }> = ({ address }) => {
  const [tokens, setTokens] = useState<TokenInfo[]>([]);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    setLoading(true);
    getAllEthereumTokensBalance(address).then((response) => {
      if (response.length > 0) {
        setTokens(response);
      }
      setLoading(false);
    });
  }, [address]);

  return (
    <div className="flex flex-col items-center justify-center w-full mt-3">
      {loading ? (
        <Spinner />
      ) : (
        <>
          <h2 className="text-2xl font-bold bg-gradient-to-r from-blue-500 to-purple-500 text-transparent bg-clip-text mb-2">
            Token Portfolio
          </h2>
          <span className="text-[8px] mt-[-5px] text-gray-500 mb-2"></span>
          {tokens.map((token: TokenInfo) => (
            <EthToken key={token.token_address} token={token} />
          ))}
        </>
      )}
    </div>
  );
};

export default EthTokenBalances;
