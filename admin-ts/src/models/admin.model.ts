import { PrismaClient as AdminPrismaClient } from "admin-client";
import { DMMFClass } from "@prisma/client/runtime";

export const AdminDB = new AdminPrismaClient();
const admin_dmmf = (AdminDB as any)._baseDmmf as DMMFClass;

export const AdminModel = (table: string) => ({
  model: admin_dmmf.modelMap[table],
  client: AdminDB,
});
