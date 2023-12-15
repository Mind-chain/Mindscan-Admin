import React, { useState } from "react";
import {
  Box,
  H3,
  Input,
  Button,
  DrawerContent,
  DrawerFooter,
} from "@adminjs/design-system";
import { ActionProps } from "adminjs";
import { ApiClient } from "adminjs";
import { useNavigate } from "react-router";

const api = new ApiClient();

const RequireUpdate = (props: ActionProps) => {
  const { record: initialRecord, resource, action } = props;
  const navigate = useNavigate();
  const [adminComments, setAdminComments] = useState("");
  const [loading, setLoading] = useState(false);

  const handleSubmit = (event: React.FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    if (adminComments.length > 0) {
      console.log("set waiting for update status to submission");
      setLoading(true);
      api
        .recordAction({
          recordId: initialRecord.id,
          resourceId: resource.id,
          actionName: "mark_as_require_updates",
          params: {
            adminComments: adminComments,
          },
        })
        .then((response) => {
          if (response.data.redirectUrl) {
            navigate(response.data.redirectUrl);
          }
        })
        .finally(() => {
          setLoading(false);
        });
    }
  };

  return (
    <Box as="form" flex onSubmit={handleSubmit}>
      <Box
        variant="white"
        width={1 / 2}
        boxShadow="card"
        mr="xxl"
        flexShrink={0}
      >
        <DrawerContent>
          <H3>Please, specify reason of update</H3>
          <Input
            value={adminComments}
            onChange={(e) => setAdminComments(e.target.value)}
            placeholder="Update reason"
            width={1}
            variant="xl"
          ></Input>
        </DrawerContent>
        <DrawerFooter>
          <Button type="submit" variant="warn" size="lg" disabled={loading}>
            Request update
          </Button>
        </DrawerFooter>
      </Box>
    </Box>
  );
};

export default RequireUpdate;
