import { BasePropertyComponent, EditPropertyProps } from "adminjs";
import React, { FC } from "react";

const IconUrlEdit: FC<EditPropertyProps> = (props) => {
  const { property } = props;
  const DefaultEdit = BasePropertyComponent.DefaultType.Edit;
  property.label = "Icon url (At least 64x64)";
  return <DefaultEdit {...props}></DefaultEdit>;
};
export default IconUrlEdit;
