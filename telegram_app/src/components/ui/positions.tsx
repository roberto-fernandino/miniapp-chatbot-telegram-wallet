import React, { useEffect, useState } from "react";
import { formatNumber } from "../../lib/utils";
import { Spinner } from "./spinner";

interface Position {
  id: string;
  tg_user_id: string;
  token_address: string;
  take_profits: [number, number][];
  stop_losses: [number, number][];
  amount: number;
  mc_entry: number;
  entry_price: number;
  created_at: string;
  chat_id: string;
}

interface PositionsProps {
  userTgId: string;
}

const Positions: React.FC<PositionsProps> = ({ userTgId }) => {
  const [positions, setPositions] = useState<Position[]>([]);
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const fetchPositions = async () => {
    try {
      const response = await fetch(
        `https://srv617785.hstgr.cloud/bot_api/positions/${userTgId}`
      );
      if (!response.ok) {
        throw new Error("Failed to fetch positions");
      }
      const data = await response.json();
      setPositions(data);
      setError(null);
    } catch (err) {
      setError(
        err instanceof Error ? err.message : "Failed to fetch positions"
      );
    } finally {
      setIsLoading(false);
    }
  };

  useEffect(() => {
    fetchPositions();
    const interval = setInterval(fetchPositions, 10000); // Fetch every 10 seconds

    return () => clearInterval(interval);
  }, [userTgId]);

  if (error) {
    return <div className="text-red-500 text-sm">{error}</div>;
  }

  return (
    <div className="w-full">
      <h2 className="text-xl font-semibold mb-4">Your Positions</h2>
      {isLoading ? (
        <div className="flex justify-center">
          <Spinner />
        </div>
      ) : positions.length === 0 ? (
        <p className="text-gray-500 text-center">No active positions</p>
      ) : (
        <div className="space-y-3">
          {positions.map((position) => (
            <div
              key={position.id}
              className="bg-gray-50 rounded-lg p-4 shadow-sm"
            >
              <div className="flex justify-between items-center">
                <div>
                  <h3 className="font-medium">$SYMBOL</h3>
                  <p className="text-sm text-gray-500">
                    {position.token_address.slice(0, 6)}...
                    {position.token_address.slice(-4)}
                  </p>
                </div>
                <div className="text-right">
                  <p className="font-medium">
                    {formatNumber(position.amount)} tokens
                  </p>
                </div>
              </div>
              <div className="mt-2 text-sm text-gray-600 flex justify-between">
                <span>Entry: ${formatNumber(position.entry_price)}</span>
              </div>
            </div>
          ))}
        </div>
      )}
    </div>
  );
};

export default Positions;
