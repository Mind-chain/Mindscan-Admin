import { BasePropertyProps } from "adminjs";
import React from "react";
import { ExtractChainsFromProps } from "./select";
import { ValueGroup } from "@adminjs/design-system";

const ChainIdsShow = (props: BasePropertyProps) => {
  const [chains, setChains] = React.useState("");
  React.useEffect(() => {
    const chains = ExtractChainsFromProps(props)
      .map(({ label }) => label)
      .join(", ");
    setChains(chains);
  }, []);
  return (
    <ValueGroup label="Chains">
      <div>{chains}</div>
    </ValueGroup>
  );
};

export default ChainIdsShow;
