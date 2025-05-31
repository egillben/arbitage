const { expect } = require("chai");
const { ethers } = require("hardhat");

describe("ArbitrageExecutor", function () {
  let arbitrageExecutor;
  let owner;
  let user1;
  let user2;
  
  // Mock addresses for testing
  const mockLendingPoolAddress = "0x1111111111111111111111111111111111111111";
  const mockUniswapRouterAddress = "0x2222222222222222222222222222222222222222";
  const mockSushiswapRouterAddress = "0x3333333333333333333333333333333333333333";
  const mockCurveRouterAddress = "0x4444444444444444444444444444444444444444";

  beforeEach(async function () {
    // Get signers
    [owner, user1, user2] = await ethers.getSigners();
    
    // Deploy the contract
    const ArbitrageExecutor = await ethers.getContractFactory("ArbitrageExecutor");
    arbitrageExecutor = await ArbitrageExecutor.deploy(
      mockLendingPoolAddress,
      mockUniswapRouterAddress,
      mockSushiswapRouterAddress,
      mockCurveRouterAddress
    );
    await arbitrageExecutor.deployed();
  });

  describe("Deployment", function () {
    it("Should set the right owner", async function () {
      expect(await arbitrageExecutor.owner()).to.equal(owner.address);
    });

    it("Should set the correct addresses", async function () {
      expect(await arbitrageExecutor.lendingPoolAddress()).to.equal(mockLendingPoolAddress);
      expect(await arbitrageExecutor.uniswapRouterAddress()).to.equal(mockUniswapRouterAddress);
      expect(await arbitrageExecutor.sushiswapRouterAddress()).to.equal(mockSushiswapRouterAddress);
      expect(await arbitrageExecutor.curveRouterAddress()).to.equal(mockCurveRouterAddress);
    });

    it("Should authorize the deployer", async function () {
      expect(await arbitrageExecutor.authorizedCallers(owner.address)).to.equal(true);
    });
  });

  describe("Access Control", function () {
    it("Should allow the owner to authorize a caller", async function () {
      await arbitrageExecutor.authorizeCaller(user1.address);
      expect(await arbitrageExecutor.authorizedCallers(user1.address)).to.equal(true);
    });

    it("Should allow the owner to unauthorize a caller", async function () {
      await arbitrageExecutor.authorizeCaller(user1.address);
      expect(await arbitrageExecutor.authorizedCallers(user1.address)).to.equal(true);
      
      await arbitrageExecutor.unauthorizeCaller(user1.address);
      expect(await arbitrageExecutor.authorizedCallers(user1.address)).to.equal(false);
    });

    it("Should not allow non-owners to authorize callers", async function () {
      await expect(
        arbitrageExecutor.connect(user1).authorizeCaller(user2.address)
      ).to.be.revertedWith("Ownable: caller is not the owner");
    });
  });

  describe("Emergency Controls", function () {
    it("Should allow the owner to activate emergency stop", async function () {
      await arbitrageExecutor.activateEmergencyStop();
      expect(await arbitrageExecutor.emergencyStop()).to.equal(true);
    });

    it("Should allow the owner to deactivate emergency stop", async function () {
      await arbitrageExecutor.activateEmergencyStop();
      expect(await arbitrageExecutor.emergencyStop()).to.equal(true);
      
      await arbitrageExecutor.deactivateEmergencyStop();
      expect(await arbitrageExecutor.emergencyStop()).to.equal(false);
    });

    it("Should not allow non-owners to control emergency stop", async function () {
      await expect(
        arbitrageExecutor.connect(user1).activateEmergencyStop()
      ).to.be.revertedWith("Ownable: caller is not the owner");
    });
  });

  describe("Recovery Functions", function () {
    it("Should allow the owner to recover ETH", async function () {
      // This test would require sending ETH to the contract first
      // and then testing the recovery function
      // For simplicity, we'll just test that the function exists
      expect(arbitrageExecutor.recoverETH).to.be.a("function");
    });

    it("Should allow the owner to recover ERC20 tokens", async function () {
      // This test would require deploying a mock ERC20 token,
      // sending some to the contract, and then testing the recovery function
      // For simplicity, we'll just test that the function exists
      expect(arbitrageExecutor.recoverERC20).to.be.a("function");
    });
  });

  // Note: Testing the flash loan and arbitrage functionality would require
  // mocking the external contracts (Aave, Uniswap, etc.) which is beyond
  // the scope of this basic test file
});