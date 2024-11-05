import React, { useState } from "react";
import Slider from "rc-slider";
import "rc-slider/assets/index.css";
import { Position } from "./positions";
import axios from "axios";
import { BOT_API_URL } from "../../lib/utils";

export const SellPercentageModal: React.FC<{
  isOpen: boolean;
  onClose: () => void;
  selectedPosition: Position | null;
}> = ({ isOpen, onClose, selectedPosition }) => {
  const [selectedPercentage, setSelectedPercentage] = useState(0);

  const handleButtonClick = (percentage: number) => {
    setSelectedPercentage(percentage);
  };

  const sell = async (position: Position | null, percentage: number) => {
    if (position) {
      axios.post(`${BOT_API_URL}/sell/position`, {
        token_address: position.token_address,
        user_tg_id: position.tg_user_id,
        sell_percentage: percentage,
      });
      onClose();
    }
  };
  const handleSliderChange = (value: number | number[]) => {
    if (Array.isArray(value)) {
      setSelectedPercentage(value[0]);
    } else {
      setSelectedPercentage(value);
    }
  };

  if (!isOpen) return null;

  return (
    <div className="fixed inset-0 bg-gray-800 bg-opacity-50 flex justify-center items-center">
      <div className="bg-white p-6 rounded-lg shadow-lg w-80">
        <h3 className="text-lg font-semibold mb-4">Select Sell Percentage</h3>
        <div className="grid grid-cols-5 gap-2 mb-4">
          {[10, 20, 30, 40, 50].map((percentage) => (
            <button
              key={percentage}
              className={`p-2 rounded-md ${
                selectedPercentage === percentage
                  ? "bg-blue-500 text-white"
                  : "bg-gray-200"
              }`}
              onClick={() => handleButtonClick(percentage)}
            >
              {percentage}%
            </button>
          ))}
        </div>
        <div className="grid grid-cols-5 gap-2 mb-4">
          {[60, 70, 80, 90, 100].map((percentage) => (
            <button
              key={percentage}
              className={`p-2 rounded-md ${
                selectedPercentage === percentage
                  ? "bg-blue-500 text-white"
                  : "bg-gray-200"
              }`}
              onClick={() => handleButtonClick(percentage)}
            >
              {percentage}%
            </button>
          ))}
          <div className="mb-4">
            <Slider
              className="horizontal-slider"
              value={selectedPercentage}
              onChange={handleSliderChange}
              min={0}
              max={100}
            />
            <div className="text-center mt-2">{selectedPercentage}%</div>
          </div>
        </div>
        <div className="flex justify-end">
          <button
            className="bg-red-500 text-white px-4 py-2 rounded-md mr-2"
            onClick={() => sell(selectedPosition, selectedPercentage)}
          >
            Sell
          </button>
          <button
            className="bg-gray-300 text-black px-4 py-2 rounded-md"
            onClick={onClose}
          >
            Cancel
          </button>
        </div>
      </div>
    </div>
  );
};
