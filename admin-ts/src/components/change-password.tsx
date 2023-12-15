import React, { useState } from "react";
import { FormGroup, Label, Input, Button } from "@adminjs/design-system";
import { ApiClient, BasePropertyProps, useNotice } from "adminjs";
import { useNavigate } from "react-router";

const api = new ApiClient();

const ChangePassword = (props: BasePropertyProps) => {
  const { record } = props;
  const [newPassword, setNewPassword] = useState("");
  const onNotice = useNotice();
  const navigate = useNavigate();

  const handleSubmit = (e) => {
    e.preventDefault();
    api
      .recordAction({
        resourceId: "Users",
        actionName: "changePassword",
        recordId: props.record.id,
        data: {
          newPassword: newPassword,
        },
      })
      .then((response) => {
        if (response.data.notice) {
          onNotice(response.data.notice);
        }
        if (response.data.redirectUrl) {
          navigate(response.data.redirectUrl);
        }
      });
  };

  return (
    <form onSubmit={handleSubmit}>
      <FormGroup>
        <Label htmlFor="newPassword">
          New Password for user {record?.params?.email}
        </Label>
        <Input
          type="password"
          id="newPassword"
          name="newPassword"
          value={newPassword}
          onChange={(e) => setNewPassword(e.target.value)}
        />
      </FormGroup>
      <Button type="submit">Change Password</Button>
    </form>
  );
};

export default ChangePassword;
