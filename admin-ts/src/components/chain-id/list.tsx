import { BasePropertyProps } from "adminjs";
import React from "react";
import { truncateString } from "../../utils";
import { ExtractChainFromProps } from "./select";

const ChainIdList = (props: BasePropertyProps) => {
  const [chain, setChain] = React.useState(undefined);
  React.useEffect(() => {
    let chain = ExtractChainFromProps(props)?.label;
    chain = truncateString(chain, 20);
    setChain(chain);
  }, []);
  return <div>{chain}</div>;
};

export default ChainIdList;
