import { BasePropertyProps } from "adminjs";
import React from "react";
import { ExtractChainsFromProps } from "./select";
import { truncateString } from "../../utils";

const ChainIdsList = (props: BasePropertyProps) => {
  const [chains, setChains] = React.useState("");
  React.useEffect(() => {
    let chains = ExtractChainsFromProps(props)
      .map(({ label }) => label)
      .join(", ");
    chains = truncateString(chains, 20);
    setChains(chains);
  }, []);
  return <div>{chains}</div>;
};

export default ChainIdsList;
