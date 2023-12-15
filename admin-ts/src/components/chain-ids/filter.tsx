import React from "react";
import styled from "styled-components";
import { Select } from "@adminjs/design-system";
import { BasePropertyProps } from "adminjs";

const Container = styled.div`
  margin-bottom: 16px;
`;

const Label = styled.label`
  display: block;
  font-size: 12px;
  line-height: 16px;
  margin-bottom: 8px;
`;

const ChainIdsFilter = (props: BasePropertyProps) => {
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
    const values = selectedOptions.map(({ value }) => value);
    props.onChange("chain_ids", values);
  }, []);

  return (
    <Container>
      <Label htmlFor="chain_ids" required>
        Chains
      </Label>
      <Select
        id="chain_ids"
        options={options}
        isMulti
        variant="filter"
        onChange={handleChange}
        value={undefined}
      />
    </Container>
  );
};

export default ChainIdsFilter;
