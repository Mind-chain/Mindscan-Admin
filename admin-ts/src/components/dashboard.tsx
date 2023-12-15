import React, { useEffect } from "react";
import styled from "styled-components";
import { useNavigate } from "react-router";

const Container = styled.div`
  display: flex;
  height: 100%;
  justify-content: center;
  align-items: center;
`;

const Title = styled.h1`
  font-size: 24px;
  line-height: 30px;
  text-align: center;
`;

const Dashboard = () => {
  const navigate = useNavigate();
  useEffect(() => {
    navigate("/admin/resources/allSubmissions");
  }, []);
  return <></>;
  // return (
  //   <Container>
  //     <Title>Welcome to Blockscout Admin Console</Title>
  //   </Container>
  // );
};

export default Dashboard;
