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
  ui_amount: string;
  mc_entry: number;
  entry_price: number;
  sol_entry: number;
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
  const [sellPercentageModalOpen, setSellPercentageModalOpen] = useState(false);
  const [selectedPosition, setSelectedPosition] = useState<Position | null>(
    null
  );
  const [sellPercentage, setSellPercentage] = useState(0);

  const handleOpenSellPercentageModal = async (position: Position) => {
    setSelectedPosition(position);
    setSellPercentageModalOpen(true);
  };

  const fetchPositions = async () => {
    try {
      const response = await fetch(
        `https://srv617785.hstgr.cloud/bot_api/get_positions/${userTgId}`
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
                return position;
              }

              const currentPrice = parseFloat(scannerData.pair.pairPrice1Usd);
              const pnlPercentage =
                position.entry_price > 0
                  ? ((currentPrice - position.entry_price) /
                      position.entry_price) *
                    100
                  : position.pnlPercentage;
              const symbol = scannerData.pair?.token1Symbol || position.symbol;

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
              <div className="flex flex-col justify-between items-center">
                <div className="flex items-start ">
                  <h3 className="font-medium">{position.symbol}/SOL</h3>
                </div>
                <div className="flex flex-row items-start">
                  PNL:
                  <span
                    className={`font-medium ${
                      position.pnlPercentage && position.pnlPercentage < 0.0
                        ? "text-red-500"
                        : "text-green-500"
                    }`}
                  >
                    {position.pnlPercentage && position.pnlPercentage < 0.0
                      ? "-" +
                        (
                          position.sol_entry * (position.pnlPercentage || 0)
                        ).toFixed(6)
                      : "+" +
                        (
                          position.sol_entry * (position.pnlPercentage || 0)
                        ).toFixed(6)}
                    SOL [{position.pnlPercentage?.toFixed(2)}% ROI]
                  </span>
                </div>
                <div className="flex flex-row items-start">
                  Size: {position.sol_entry} SOL at{" "}
                  {formatNumber(position.mc_entry)}
                </div>
                <div className="text-sm text-gray-500">
                  Date: {new Date(position.created_at).toLocaleString()}
                </div>
                <div>
                  <button
                    className="bg-red-500 text-white px-2 py-1 rounded-md"
                    onClick={() => handleOpenSellPercentageModal(position)}
                  >
                    Sell
                  </button>
                </div>
              </div>
            </div>
          ))}
        </div>
      )}
    </div>
  );
};

export default Positions;
