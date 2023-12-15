import { Components } from "./components";

export const ONLY_SHOW_PROPERTY = {
  isVisible: {
    show: true,
    edit: false,
    new: false,
    filter: false,
  },
  isDisabled: true,
};

export const ID_PROPERTY = {
  ...ONLY_SHOW_PROPERTY,
  isId: true,
};

export const CREATE_CHAIN_ID_PRORERTY = (networks: any) => {
  return {
    type: "string",
    components: {
      edit: Components.ChainId.Select,
      new: Components.ChainId.Select,
      list: Components.ChainId.List,
      show: Components.ChainId.Show,
    },
    props: {
      networks,
    },
  };
};

export const PROJECT_SECTOR_PROPERTY = {
  type: "string",
  components: {
    edit: Components.ProjectSector,
    add: Components.ProjectSector,
  },
};
