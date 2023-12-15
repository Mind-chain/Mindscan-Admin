import { ComponentLoader } from "adminjs";

const componentLoader = new ComponentLoader();

const Components = {
  RejectSubmission: componentLoader.add(
    "RejectSubmission",
    "./components/reject-submission"
  ),
  RequireUpdate: componentLoader.add(
    "RequireUpdate",
    "./components/require-update"
  ),
  Dashboard: componentLoader.add("Dashboard", "./components/dashboard"),
  Status: {
    Show: componentLoader.add("Show", "./components/status/show"),
    List: componentLoader.add("List", "./components/status/list"),
  },
  ProjectSector: componentLoader.add(
    "ProjectSector",
    "./components/project-sector"
  ),
  UrlPropery: componentLoader.add("UrlPropery", "./components/url-property"),
  ImagePropery: componentLoader.add(
    "ImagePropery",
    "./components/image-property"
  ),
  ChainIds: {
    Select: componentLoader.add(
      "ChainIdsSelect",
      "./components/chain-ids/select"
    ),
    List: componentLoader.add("ChainIdsList", "./components/chain-ids/list"),
    Show: componentLoader.add("ChainIdsShow", "./components/chain-ids/show"),
    Filter: componentLoader.add(
      "ChainIdsFilter",
      "./components/chain-ids/filter"
    ),
  },
  ChainId: {
    Select: componentLoader.add(
      "ChainIdSelectSingle",
      "./components/chain-id/select"
    ),
    List: componentLoader.add("ChainIdList", "./components/chain-id/list"),
    Show: componentLoader.add("ChainIdShow", "./components/chain-id/show"),
  },
  ChangePassword: componentLoader.add(
    "ChangePasswordComponent",
    "./components/change-password"
  ),
  IconUrl: {
    Edit: componentLoader.add("IconUrlEdit", "./components/icon-url/edit"),
  },
  DisabledEditText: componentLoader.add(
    "DisabledEditText",
    "./components/disabled-edit-text"
  ),
  Duplicate: componentLoader.add("Duplicate", "./components/duplicate"),
};

export { componentLoader, Components };
