import { ActionContext, ActionRequest, ValidationError } from "adminjs";

export const UniqueChainIdAddressValidator = async (
  request: ActionRequest,
  context: ActionContext
) => {
  const { payload = {}, method } = request;
  const { resource } = context;
  // We only want to validate "post" requests
  if (method !== "post") return request;
  const { address = "", chain_id = "" } = payload;
  if (
    await (resource as any).manager.findFirst({
      where: {
        address: address.toLowerCase(),
        chain_id: BigInt(chain_id),
      },
    })
  ) {
    throw new ValidationError({
      address: {
        message: "This address is already used for this chain",
      },
    });
  }
  return request;
};

export const FieldsNonEmptyValidatorBuilder = (fields: string[]) => {
  return async (request: ActionRequest, context: ActionContext) => {
    const { method, payload } = request;
    if (method === "post") {
      const errors = {};
      for (const i in fields) {
        if (!payload[fields[i]]) {
          errors[fields[i]] = {
            message: "Field is required",
            type: "required",
          };
        }
      }
      if (Object.keys(errors).length) {
        throw new ValidationError(errors);
      }
    }
    return request;
  };
};
