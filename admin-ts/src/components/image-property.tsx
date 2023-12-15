import { BasePropertyProps } from "adminjs";
import React from "react";
import UrlProperty from "./url-property";

const ImagePropery = (props: BasePropertyProps) => {
  return <UrlProperty {...props} isImage />;
};

export default ImagePropery;
