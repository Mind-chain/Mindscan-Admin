import React from "react";
import { Input, Label, Select } from "@adminjs/design-system";
import { BasePropertyProps } from "adminjs";
import styled from "styled-components";

const Container = styled.div`
  margin-bottom: 16px;
`;

interface SelectorsResponse {
  projectSectors: string[];
}

interface Option {
  value: string;
  label: string;
}

const option = (name: string) => ({
  value: name,
  label: name,
});

const ProjectSector = (props: BasePropertyProps) => {
  const ADMIN_RS_HOST = (window as any).AdminJS.env.ADMIN_RS_HOST;
  const [options, setOptions] = React.useState([]);
  const [value, setValue] = React.useState(undefined);

  React.useEffect(() => {
    const project_sector = props.record.params.project_sector;
    console.log("project_sector", project_sector, props.record.params);

    setValue(option(project_sector));
  }, []);

  React.useEffect(() => {
    const chain_id = props.record.params.chain_id || "0";
    const url = `${ADMIN_RS_HOST}/api/v1/chains/${chain_id}/token-info-submissions/selectors`;
    fetch(url)
      .then((r) => r.json())
      .then((response: SelectorsResponse) => {
        const options = response.projectSectors.map(option);
        setOptions(options);
      });
  }, []);

  const handleChange = React.useCallback((option_value: Option) => {
    props.onChange("project_sector", option_value?.value || null);
    setValue(option_value);
  }, []);

  return (
    <Container>
      <Label htmlFor="project-sector" required={false}>
        Project sector
      </Label>
      <Select
        id="project-sector"
        options={options}
        required={false}
        isMulti={false}
        onChange={handleChange}
        value={value}
      />
    </Container>
  );
};

export default ProjectSector;
