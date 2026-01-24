# pallet_xcavate_whitelist

## Overview

This pallet is a core component of the Xcavate protocol.

The pallet xcavate whitelist module manages role-based permissions and the compliance status for users in the Xcavate ecosystem. It enables administrators to assign, revoke and update roles and ensuring only authorized and compliant users can perform actions in pallets related to the Xcavate real estate ecosystem. The pallet integrates with KYC/AML providers to verify user credentials.

### Key Concepts

#### Roles and Permissions

Each user can be assigned multiple roles, each with a compliance status (`Compliant` or `Revoked`).  
Roles gate access to actions in other pallets (e.g., `buy_property_token` in `pallet_marketplace`).  
Compliance is verified via KYC/AML credentials, ensuring only users with permission can act.

#### Role System

The system defines six roles:

- **RegionalOperator**: Oversees regional operations.
- **RealEstateInvestor**: Buys and invests in real estate assets.
- **RealEstateDeveloper**: Creates and sells real estate projects.
- **Lawyer**: Handles legal aspects of real estate.
- **LettingAgent**: Manages rental properties and distributes income.
- **SpvConfirmation**: Confirms Special Purpose Vehicles (SPVs).

#### Permission Management

Admins manage roles and compliance, while sudo controls admin accounts.  

## Storage Items

- **AdminAccounts**: Maps admin accounts to an empty tuple.
- **AccountRoles**: Maps (account, role) to compliance status (`Compliant`/`Revoked`).

## Events

- **AdminRegistered**: A new admin has been added.
- **AdminRemoved**: An admin has been removed.
- **RoleAssigned**: A role has been assigned to a user.
- **RoleRemoved**: A role has been removed from a user.
- **PermissionUpdated**: A user’s compliance status has been updated.

## Errors

- **AlreadyAdmin**: The account is already registered as an admin.
- **AccountNotAdmin**: The account is not registered as an admin.
- **RoleAlreadyAssigned**: The role has already been assigned to the user.
- **RoleNotAssigned**: The role has not been assigned to the user.
- **PermissionAlreadySet**: This permission has already been set.

## Config

- **RuntimeEvent**: Type of runtime events emitted by the pallet.
- **WeightInfo**: Type representing the weight of this pallet.
- **WhitelistOrigin**: Origin check for admin actions (e.g., sudo).

## Extrinsics

- `add_admin`: Add a new whitelist admin (sudo only).
- `remove_admin`: Remove an existing whitelist admin (sudo only).
- `assign_role`: Assign a role to a user with default 'Compliant' permission.
- `remove_role`: Remove a role from a user.
- `set_permission`: Update a user's permission for a role.

## Trait Implementation

The pallet provides the `RolePermission` trait for integration with other pallets:
```rust
pub trait RolePermission<AccountId> {
    fn has_role(account: &AccountId, role: Role) -> bool;
    fn is_compliant(account: &AccountId, role: Role) -> bool;
    fn is_admin(account: &AccountId) -> bool;
}
```

- `has_role`: Verifies if an account has a role, such as LettingAgent for calling certain extrinsics.
- `is_compliant`: Ensures an account has a role and is compliant.
- `is_admin`: Confirm admin status for role management.