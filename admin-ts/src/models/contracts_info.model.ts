import { PrismaClient as ContractsInfoPrismaClient } from "contracts-info";
import { DMMFClass } from "@prisma/client/runtime";

export const ContractsInfoDB = new ContractsInfoPrismaClient();
const contracts_info_dmmf = (ContractsInfoDB as any)._baseDmmf as DMMFClass;

export const ContractsInfoModel = (table: string) => ({
  model: contracts_info_dmmf.modelMap[table],
  client: ContractsInfoDB,
});
