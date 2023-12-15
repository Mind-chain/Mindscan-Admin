import React, { FC } from "react";
import { ShowPropertyProps } from "adminjs";
import { Badge, Box, Tooltip } from "@adminjs/design-system";

const variantByStatus = (status: string) => {
  if (status == "IN_PROCESS") {
    return "info";
  } else if (status == "WAITING_FOR_UPDATE") {
    return "default";
  } else if (status == "APPROVED") {
    return "success";
  } else if (status == "REJECTED") {
    return "danger";
  } else {
    return "default";
  }
};

type WrapperProps = ShowPropertyProps & {
  useTooltip?: boolean;
};

const StatusPropertyValue: FC<WrapperProps> = (props) => {
  const { record, property, useTooltip = false } = props;

  const rawValue = record?.params[property.path];

  if (typeof rawValue === "undefined" || rawValue === "") {
    return null;
  }

  const variant = variantByStatus(rawValue);
  const badge = (
    <Badge variant={variant} outline>
      {rawValue}
    </Badge>
  );
  const adminComments = record?.params.admin_comments;
  if (adminComments) {
    let title = "REASON: '" + adminComments + "'";
    if (useTooltip) {
      return (
        <Tooltip direction="top" title={title} size="default">
          {badge}
        </Tooltip>
      );
    } else {
      return (
        <Box flex>
          <Box flex flexDirection="column">
            <Box mb="sm">{badge}</Box>
            <Box>{title}</Box>
          </Box>
        </Box>
      );
    }
  } else {
    return badge;
  }
};

export default StatusPropertyValue;
