import React, { useState } from "react";
import styled from "styled-components";
import { FormGroup, FormMessage, Label, Select } from "@adminjs/design-system";
import {
  ApiClient,
  BasePropertyComponent,
  BasePropertyProps,
  useCurrentAdmin,
} from "adminjs";

const Container = styled.div`
  margin-bottom: 16px;
`;

interface Option {
  value: string;
  label: string;
}

const api = new ApiClient();

export const ExtractChainFromProps = (props: BasePropertyProps) => {
  const networks = props.property.props.networks;
  const chain_id = props.record.params.chain_id;
  if (chain_id) {
    return Object.entries(networks)
      .map(([key, value]: [string, { label: string }]) => ({
        value: key,
        label: value.label,
      }))
      .find(({ value }) => chain_id == value);
  }
};

const ChainIdSelect = (props: BasePropertyProps) => {
  const [value, setValue] = React.useState(undefined);
  const [options, setOptions] = useState([]);
  const { property, record } = props;
  const error = record.errors?.[property.path];

  React.useEffect(() => {
    const initialChain = ExtractChainFromProps(props);
    setValue(initialChain);
  }, []);

  React.useEffect(() => {
    api
      .resourceAction({
        resourceId: "Users",
        actionName: "availableNetworks",
      })
      .then((response) => {
        const possibleNetworks = response.data.networks;
        const networks = props.property.props.networks;
        let options = Object.entries(networks).map(
          ([key, value]: [string, { label: string }]) => ({
            value: key,
            label: value.label,
          })
        );
        if (response.data.networks) {
          options = options.filter((network) =>
            possibleNetworks.includes(Number(network.value))
          );
        }
        console.log("network options: ", options);
        setOptions(options);
      });
  }, []);

  const handleChange = React.useCallback((maybe_value: Option | undefined) => {
    props.onChange("chain_id", maybe_value?.value || null);
    setValue(maybe_value);
  }, []);
  const isDisabled = Boolean(property.isDisabled && record.id);
  return (
    <FormGroup error={Boolean(error)}>
      <Label htmlFor="chain_id" required={true}>
        Chain
      </Label>
      <Select
        id="chain_id"
        required={true}
        options={options}
        isMulti={false}
        onChange={handleChange}
        isDisabled={isDisabled}
        value={value}
      />
      <FormMessage>{error && error.message}</FormMessage>
    </FormGroup>
  );
};

export default ChainIdSelect;
