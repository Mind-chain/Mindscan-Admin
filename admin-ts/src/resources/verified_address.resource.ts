import { CreateAddressFilter } from "../utils";
import { ContractsInfoModel, ContractsInfoDB } from "../models";
import { ActionContext, ActionRequest, ValidationError } from "adminjs";
import { OnlyOwnerAccess, OnlySuperAdminAccess } from "../permissions";
import { CreateOnlyOwnerListHandler } from "../handlers";
import {
  FieldsNonEmptyValidatorBuilder,
  UniqueChainIdAddressValidator,
} from "../validators";
import {
  CREATE_CHAIN_ID_PRORERTY,
  ID_PROPERTY,
  ONLY_SHOW_PROPERTY,
} from "../fields";
import { useNetworks } from "../networks";
import { Components } from "../components";

export const CreateVerifiedAddressesResource = () => {
  const networks = useNetworks();

  return {
    resource: ContractsInfoModel("VerifiedAddress"),
    options: {
      id: "verifiedAddresses",
      navigation: {
        name: "Token services",
      },
      properties: {
        id: ID_PROPERTY,
        created_at: ONLY_SHOW_PROPERTY,
        chain_id: {
          ...CREATE_CHAIN_ID_PRORERTY(networks),
          isDisabled: {
            edit: true,
            new: false,
          },
        },
        address: {
          isDisabled: {
            edit: true,
            new: false,
          },
          components: {
            edit: Components.DisabledEditText,
          },
        },
        verified_manually: {
          isDisabled: true,
          isVisible: {
            list: true,
            edit: false,
            filter: true,
            show: true,
          },
        },
      },
      actions: {
        list: {
          handler: CreateOnlyOwnerListHandler(ContractsInfoDB, "chain_id"),
          before: [CreateAddressFilter("address")],
        },
        show: {
          before: [OnlyOwnerAccess],
        },
        new: {
          // Set the `verified_manually` parameter.
          before: [
            OnlyOwnerAccess,
            FieldsNonEmptyValidatorBuilder(["chain_id"]),
            UniqueChainIdAddressValidator,
            verifiedAddressesVerifyForm,
            verifiedAddressesConvertAddress,
            verifiedAddressesSetVerifiedManuallyTrue,
          ],
        },
        edit: {
          isAccessible: (context) => {
            const { record } = context;
            return (
              OnlySuperAdminAccess(context) | record?.params?.verified_manually
            );
          },
          before: [
            OnlyOwnerAccess,
            FieldsNonEmptyValidatorBuilder(["chain_id"]),
            verifiedAddressesVerifyForm,
            verifiedAddressesConvertAddress,
          ],
        },
        delete: {
          isAccessible: (context: ActionContext) => {
            const { record } = context;
            return (
              OnlySuperAdminAccess(context) | record?.params?.verified_manually
            );
          },
          before: [OnlyOwnerAccess],
          guard: "Delete address ownership?",
        },
        bulkDelete: {
          isAccessible: false,
        },
      },
    },
  };
};

const verifiedAddressesVerifyForm = async (request: ActionRequest) => {
  const { payload = {}, method } = request;

  // We only want to validate "post" requests
  if (method !== "post") return request;

  // Payload contains data sent from the frontend
  const { address = "", owner_email = "" } = payload;

  // We will store validation errors in an object, so that
  // we can throw multiple errors at the same time
  const errors = {};

  // We are doing validations and assigning errors to "errors" object

  // Remove the '0x' prefix if exists (we will append it later).
  const address_trimmed = address.trim().replace(/^0x/, "");
  if (address_trimmed.length !== 40) {
    errors["address"] = {
      message: "Must be either 40 or 42 (if prefixed) length valid hex string",
    };
  } else if (address_trimmed.match(/^[0-9A-Fa-f]*$/g) === null) {
    //
    errors["address"] = {
      message: "Must be a valid hex string",
    };
  }

  if (owner_email.length === 0) {
    errors["owner_email"] = {
      message: "Must not be empty",
    };
  }

  // We throw AdminJS ValidationError if there are errors in the payload
  if (Object.keys(errors).length) {
    throw new ValidationError(errors);
  }

  return request;
};

const verifiedAddressesConvertAddress = (request: ActionRequest) => {
  const { payload = {}, method } = request;

  // We only want to validate "post" requests
  if (method !== "post") return request;

  // Should be verified before that address is valid 40 or 42 length hex string

  let { address: converted_address } = payload;

  if (converted_address) {
    if (converted_address.length !== 42) {
      converted_address = "0x" + converted_address;
    }
    converted_address = converted_address.toLowerCase();

    request.payload = {
      ...request.payload,
      address: converted_address,
    };
  }

  return request;
};

const verifiedAddressesSetVerifiedManuallyTrue = (request: ActionRequest) => {
  if (request?.payload) {
    request.payload = {
      ...request.payload,
      verified_manually: true,
    };
  }
  return request;
};
