import React from "react";
import StatusPropertyValue from "./status-propery-value";
import { ShowPropertyProps } from "adminjs";

const List: React.FC<ShowPropertyProps> = (props) => (
  <StatusPropertyValue {...props} useTooltip />
);

export default List;
