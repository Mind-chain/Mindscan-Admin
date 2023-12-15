import { ValueGroup, Link, Box } from "@adminjs/design-system";
import { BasePropertyProps } from "adminjs";
import React from "react";

type WrapperProps = BasePropertyProps & {
  isImage: boolean;
};

export const isValidHttpUrl = (s: string) => {
  let url;
  try {
    url = new URL(s);
  } catch (_) {
    return false;
  }
  return url.protocol === "http:" || url.protocol === "https:";
};

const UrlProperty = (props: WrapperProps) => {
  const { property, record, isImage = false } = props;
  const rawValue = record?.params[property.path];

  if (typeof rawValue === "undefined") {
    return null;
  }
  const isLink = isValidHttpUrl(rawValue);

  let value: any;
  if (!isLink) {
    value = rawValue;
  } else {
    let preview;
    if (isImage) {
      preview = <img src={rawValue} alt={rawValue} />;
    } else {
      preview = rawValue;
    }
    value = (
      <Link target="_blank" variant="primary" href={rawValue}>
        {preview}
      </Link>
    );
  }

  return <ValueGroup label={property.label}>{value}</ValueGroup>;
};

export default UrlProperty;
