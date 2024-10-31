import React, { useEffect, useState, useCallback } from "react";
import { getScanner } from "../../lib/utils";
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
  currentPrice?: number;
  pnlPercentage?: number;
  symbol?: string;
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

  const updatePositionsWithPrices = useCallback(
    async (currentPositions: Position[]) => {
      try {
        const updatedPositions = await Promise.all(
          currentPositions.map(async (position) => {
            try {
              const scannerData = await getScanner(position.token_address);

              if (!scannerData?.pair?.pairPrice1Usd) {
                return position; // Keep existing position data if price fetch fails
              }

              const currentPrice = parseFloat(scannerData.pair.pairPrice1Usd);
              const pnlPercentage =
                ((currentPrice - position.entry_price) / position.entry_price) *
                100;
              const symbol = scannerData.pair?.token0Symbol || "Unknown";

              return {
                ...position,
                currentPrice,
                pnlPercentage,
                symbol,
              };
            } catch (error) {
              console.error(`Error updating position ${position.id}:`, error);
              return position; // Keep existing position data if update fails
            }
          })
        );
        setPositions(updatedPositions);
      } catch (err) {
        console.error("Error updating prices:", err);
      }
    },
    []
  );

  useEffect(() => {
    fetchPositions();
    const positionsInterval = setInterval(fetchPositions, 10000);

    return () => clearInterval(positionsInterval);
  }, [userTgId]);

  useEffect(() => {
    if (positions.length > 0) {
      let mounted = true;

      const updatePrices = async () => {
        if (mounted) {
          await updatePositionsWithPrices(positions);
        }
      };

      updatePrices();
      const priceInterval = setInterval(updatePrices, 5000);

      return () => clearInterval(priceInterval);
    }
  }, [positions.length, updatePositionsWithPrices]);

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
                  <h3 className="font-medium"></h3>
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
              <div className="mt-2 text-sm flex justify-between items-center">
                <span>Entry: ${formatNumber(position.entry_price)}</span>
                {position.pnlPercentage !== undefined && (
                  <span
                    className={`font-medium ${
                      position.pnlPercentage >= 0
                        ? "text-green-500"
                        : "text-red-500"
                    }`}
                  >
                    {position.pnlPercentage >= 0 ? "+" : ""}
                    {formatNumber(position.pnlPercentage)}%
                  </span>
                )}
              </div>
            </div>
          ))}
        </div>
      )}
    </div>
  );
};

export default Positions;
