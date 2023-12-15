import React from "react";
import styled from "styled-components";
import { Label, Select } from "@adminjs/design-system";
import { BasePropertyProps } from "adminjs";

const Container = styled.div`
  margin-bottom: 16px;
`;

const ErrorMessageContainer = styled.div`
  height: 24px;
  margin-top: 4px;
  color: red;
  font-size: 12px;
  line-height: 16px;
`;

export const ExtractChainsFromProps = (props: BasePropertyProps) => {
  const networks = props.property.props.networks;
  const chain_ids = props.record.params.chain_ids;
  if (chain_ids) {
    return Object.entries(networks)
      .map(([key, value]: [string, { label: string }]) => ({
        value: key,
        label: value.label,
      }))
      .filter(({ value }) =>
        props.record.params.chain_ids.includes(Number(value))
      );
  } else {
    return [];
  }
};

const ChainIdsSelect = (props: BasePropertyProps) => {
  const [chainIds, setChainIds] = React.useState([]);
  React.useEffect(() => {
    const initialChainIds = ExtractChainsFromProps(props);
    setChainIds(initialChainIds);
  }, []);

  const options = React.useMemo(() => {
    const networks = props.property.props.networks;
    return Object.entries(networks).map(
      ([key, value]: [string, { label: string }]) => ({
        value: key,
        label: value.label,
      })
    );
  }, []);

  const handleChange = React.useCallback((selectedOptions) => {
    setChainIds(selectedOptions);
    const values = selectedOptions.map(({ value }) => value);
    props.onChange("chain_ids", values);
  }, []);

  const isRequired = !props.record?.params?.is_superuser;

  return (
    <Container>
      <Label htmlFor="chain_ids" required={isRequired}>
        Chains
      </Label>
      <Select
        id="chain_ids"
        options={options}
        isMulti
        onChange={handleChange}
        value={chainIds}
      />
      <ErrorMessageContainer>
        {props.record?.errors?.chain_ids?.message || ""}
      </ErrorMessageContainer>
    </Container>
  );
};

export default ChainIdsSelect;
