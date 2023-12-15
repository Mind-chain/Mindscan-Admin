import React from "react";
import { ValueGroup } from "@adminjs/design-system";
import StatusPropertyValue from "./status-propery-value";
import { ShowPropertyProps } from "adminjs";

const Show: React.FC<ShowPropertyProps> = (props) => {
  const { property } = props;
  return (
    <ValueGroup label={property.label}>
      <StatusPropertyValue {...props} />
    </ValueGroup>
  );
};

export default Show;
