import { BasePropertyComponent, EditPropertyProps } from "adminjs";
import React, { FC } from "react";

const DisabledEditText: FC<EditPropertyProps> = (props) => {
  const { property, record } = props;
  const DefaultEdit = BasePropertyComponent.DefaultType.Edit;
  // disabled only for edit action, not from new action
  property.isDisabled = Boolean(property.isDisabled && record.id);
  return <DefaultEdit {...props}></DefaultEdit>;
};
export default DisabledEditText;
