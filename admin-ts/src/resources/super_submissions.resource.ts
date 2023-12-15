import { AdminModel } from "../models";
import { OnlySuperAdminAccess } from "../permissions";
import { CreateAddressFilter } from "../utils";
import { CreateSubmissionsResource } from ".";
import { FieldsNonEmptyValidatorBuilder } from "../validators";

export const CreateSuperSubmissionsResource = () => {
  const submissions_resource = CreateSubmissionsResource();

  return {
    resource: AdminModel("Submission"),
    options: {
      id: "SuperSubmissions",
      navigation: {
        name: "Super User",
      },
      properties: {
        ...submissions_resource.options.properties,
        chain_id: {
          ...submissions_resource.options.properties.chain_id,
          isDisabled: false,
        },
        token_address: {
          isDisabled: false,
        },
        blockscout_user_email: {
          isDisabled: false,
        },
        status: {
          ...submissions_resource.options.properties.status,
          isVisible: {
            new: false,
            edit: false,
            show: true,
            filter: true,
            list: true,
          },
          isDisabled: {
            new: true,
            edit: true,
          },
        },
      },
      actions: {
        list: {
          isAccessible: OnlySuperAdminAccess,
          before: [CreateAddressFilter("token_address")],
        },
        new: {
          isAccessible: OnlySuperAdminAccess,
          before: [FieldsNonEmptyValidatorBuilder(["chain_id"])],
        },
        edit: {
          isAccessible: OnlySuperAdminAccess,
          before: [FieldsNonEmptyValidatorBuilder(["chain_id"])],
        },
        show: {
          isAccessible: OnlySuperAdminAccess,
        },
      },
    },
  };
};
