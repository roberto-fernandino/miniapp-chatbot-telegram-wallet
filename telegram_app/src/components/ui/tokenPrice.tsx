import React, { useState, useEffect } from "react";
import { fetchInfo } from "../../lib/utils";

const TokenPrice: React.FC<{ ca: string }> = ({ ca }) => {
  const [price, setPrice] = useState<number | null>(null);

  useEffect(() => {
    fetchInfo(ca)
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

  if (price === null)
    return (
      <span>![] Error: Price not found in response.pair.pairPrice1Usd</span>
    );

  return <span>${price.toFixed(4)}</span>;
};

export default TokenPrice;
