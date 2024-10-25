import React, { useState, useEffect } from "react";
import { fetchDexcelerateScanner } from "../../lib/utils";
import { Spinner } from "./spinner";

const TokenPrice: React.FC<{ ca: string; amount: number }> = ({
  ca,
  amount,
}) => {
  const [price, setPrice] = useState<number | null>(null);

  useEffect(() => {
    fetchDexcelerateScanner(ca)
      .then((response) => {
        const pairPrice1Usd = response.pair?.pairPrice1Usd;
        if (pairPrice1Usd) {
          setPrice(parseFloat(pairPrice1Usd));
        } else {
          setPrice(null);
        }
      })
      .catch((error) => {
        setPrice(0);
      });
  }, [ca]);

  if (price === null) return <Spinner />;

  return <span>${(price * amount).toFixed(2)}</span>;
};

export default TokenPrice;
