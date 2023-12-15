import { BasePropertyProps } from "adminjs";
import React from "react";
import styled from "styled-components";
import { ExtractChainFromProps } from "./select";
import { ValueGroup } from "@adminjs/design-system";

const ChainIdShow = (props: BasePropertyProps) => {
  const [chain, setChain] = React.useState(undefined);
  React.useEffect(() => {
    const chain = ExtractChainFromProps(props)?.label;
    setChain(chain);
  }, []);
  return (
    <ValueGroup label="Chain">
      <div>{chain}</div>
    </ValueGroup>
  );
};

export default ChainIdShow;
