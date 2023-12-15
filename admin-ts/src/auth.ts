import { default as bcrypt } from "bcrypt";
import { AdminDB } from "./models";

export const CreateAuthOptions = () => {
  return {
    cookieName: "blockscout_admin",
    cookiePassword: process.env.COOKIE_PASSWORD || "complicatedsecurepassword",
    authenticate: async (email: string, password: string) => {
      const user = await AdminDB.user.findUnique({
        where: {
          email: email,
        },
      });
      if (user) {
        const matched = await bcrypt.compare(password, user.password);
        if (matched) {
          return user;
        }
      }
      return false;
    },
  };
};

export const CreateSessionOptions = () => {
  return {
    resave: false,
    saveUninitialized: true,
  };
};
