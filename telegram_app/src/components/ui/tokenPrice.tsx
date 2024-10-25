import React, { useState, useEffect } from "react";
import { fetchInfo } from "../../lib/utils";

const TokenPrice: React.FC<{ ca: string }> = ({ ca }) => {
  const [price, setPrice] = useState<any>(null);

  useEffect(() => {
    fetchInfo(ca).then((response) => {
      setPrice(response.pair.pairPrice1Usd);
    });
  }, [ca]);

  if (!price) return <span>Loading...</span>;

  // Assuming the price is nested under the 'ca' key
  const tokenPrice = price[ca];

  if (!tokenPrice) return <span>Price not available</span>;

  return <span>${parseFloat(tokenPrice).toFixed(4)}</span>;
};

export default TokenPrice;
