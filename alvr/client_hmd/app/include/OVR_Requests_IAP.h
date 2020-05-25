// This file was @generated with LibOVRPlatform/codegen/main. Do not modify it!

#ifndef OVR_REQUESTS_IAP_H
#define OVR_REQUESTS_IAP_H

#include "OVR_Types.h"
#include "OVR_Platform_Defs.h"

#include "OVR_ProductArray.h"
#include "OVR_PurchaseArray.h"

/// \file
/// # In-App Purchasing (IAP)
///
/// In-app purchasing allows a user to buy items without leaving VR, using
/// the credit card they have on file with Oculus.  There are two
/// different types of IAP: consumable and durable.
///
/// Durable:
/// This type is considered 'single use unlock'; once it has been
/// purchased, it cannot be purchased again. For example, you might
/// provide your app for free with a demo level and use an in-app purchase
/// to unlock the rest of the game.
///
/// Consumable:
/// This type can be purchased multiple times. Examples of consumable IAP include
/// 'character lives' or 'coins'. These items must be 'consumed' before they can be
/// purchased again. Consumable IAP items that have been purchased, but not consumed
/// will be returned as part of ovr_IAP_GetViewerPurchases().
///
/// ## Creating IAP
///
/// IAP items are defined in the developer dashboard, found at
/// https://dashboard.oculus.com/application/[YOUR_APP_ID]/iap.
///
/// Each IAP item is defined with a "SKU" (a unique identifier for
/// referring to the item), a description (shown in the checkout flow),
/// and a price.
///
/// To handle special behaviors like sales or special offers, we recommend
/// creating a new SKU (e.g "game-unlock-sale") with a custom price, and
/// use your game logic to display that to a specific set of people or for
/// a specific amount of time.
///
/// ## Main Flow
///
/// A typical use of IAP is structured like so:
///
///   1. On startup, fetch the user's current purchases with ovr_IAP_GetViewerPurchases().
///   If any of the results are consumable, consume them now.
///   2. Call ovr_IAP_GetProductsBySKU() to retrieve a list of products.
///   If you've hard-coded this list into your app and don't need to show
///   prices, this may be optional.
///   3. When the user chooses to purchase an item, call ovr_IAP_LaunchCheckoutFlow()
///   4. (for consumables): call ovr_IAP_ConsumePurchase() to consume the item, allowing later re-purchase.
///
/// ## Testing IAP
///
/// You can create a set of Test Users for your organization, which have
/// permission to purchase IAP items / applications without needing real money. These
/// users only work within applications in your organization. Test users can be created
/// at  https://dashboard.oculus.com/organizations/[YOUR_ORG_ID]/testusers. Once the
/// test user has been created, you can add special credit cards that have different
/// properties. NOTE: these cards only work with test users and they can only be used
/// within your organization. When using these cards, you can enter any
/// address and expiration date as long as it's valid.
///
/// Always succeeds:  4111 1177 1155 2927
/// Fails at auth:    4111 1180 9636 6644
/// Fails at sale:  	4111 1193 1540 5122
///
/// ## Server-to-server REST API
///
/// In some cases you would like to query for purchase information from
/// your game server rather than from the client. For example, to avoid
/// cheating you may want to maintain the user's balance of coins on the
/// server.  The available API calls are:
///
/// ### Verify the the user owns a specific IAP item
///
///     $ curl -d "access_token=$USER_ACCESSTOKEN" -d "sku=EXAMPLE1" https://graph.oculus.com/$APPID/verify_entitlement
///     {"success":true}
///
/// ### Consume an IAP item
///
///     $ curl  -d "access_token=$USER_ACCESSTOKEN" -d "sku=EXAMPLE1" https://graph.oculus.com/$APPID/consume_entitlement
///     {"success":true}
///
/// After consumption the use is no longer entitled to the item:
///
///     $ curl  -d "access_token=$USER_ACCESSTOKEN" -d "sku=EXAMPLE1" https://graph.oculus.com/$APPID/verify_entitlement
///     {"success":false}
///
/// ### Query all IAP entitlements
///
///     $ curl -G -d "access_token=$USER_ACCESSTOKEN"  -d "fields=id,item{sku}" https://graph.oculus.com/$APPID/viewer_purchases
///     {"data":[{"id":"963119010431337","item":{"sku":"EXAMPLE1"}}]}
///
/// ## Limitations
///
/// * At the moment there is no support for selling a single item with multiple currencies.
///
/// * All IAP operations require online access.  A future update will support offline caching of IAP purchases.

/// Allow the consumable IAP product to be purchased again. Conceptually, this
/// indicates that the item was used or consumed.
///
/// A message with type ::ovrMessage_IAP_ConsumePurchase will be generated in response.
///
/// First call ::ovr_Message_IsError() to check if an error occurred.
///
/// This response has no payload. If no error occured, the request was successful. Yay!
OVRP_PUBLIC_FUNCTION(ovrRequest) ovr_IAP_ConsumePurchase(const char *sku);

/// Get the next page of entries
///
/// A message with type ::ovrMessage_IAP_GetNextProductArrayPage will be generated in response.
///
/// First call ::ovr_Message_IsError() to check if an error occurred.
///
/// If no error occurred, the message will contain a payload of type ::ovrProductArrayHandle.
/// Extract the payload from the message handle with ::ovr_Message_GetProductArray().
OVRP_PUBLIC_FUNCTION(ovrRequest) ovr_IAP_GetNextProductArrayPage(ovrProductArrayHandle handle);

/// Get the next page of entries
///
/// A message with type ::ovrMessage_IAP_GetNextPurchaseArrayPage will be generated in response.
///
/// First call ::ovr_Message_IsError() to check if an error occurred.
///
/// If no error occurred, the message will contain a payload of type ::ovrPurchaseArrayHandle.
/// Extract the payload from the message handle with ::ovr_Message_GetPurchaseArray().
OVRP_PUBLIC_FUNCTION(ovrRequest) ovr_IAP_GetNextPurchaseArrayPage(ovrPurchaseArrayHandle handle);

/// Retrieve a list of IAP products that can be purchased.
/// \param skus The SKUs of the products to retrieve.
/// \param count Number of items you provided in the SKUs.
///
/// A message with type ::ovrMessage_IAP_GetProductsBySKU will be generated in response.
///
/// First call ::ovr_Message_IsError() to check if an error occurred.
///
/// If no error occurred, the message will contain a payload of type ::ovrProductArrayHandle.
/// Extract the payload from the message handle with ::ovr_Message_GetProductArray().
OVRP_PUBLIC_FUNCTION(ovrRequest) ovr_IAP_GetProductsBySKU(const char **skus, int count);

/// Retrieve a list of Purchase that the Logged-In-User has made. This list
/// will also contain consumable purchases that have not been consumed.
///
/// A message with type ::ovrMessage_IAP_GetViewerPurchases will be generated in response.
///
/// First call ::ovr_Message_IsError() to check if an error occurred.
///
/// If no error occurred, the message will contain a payload of type ::ovrPurchaseArrayHandle.
/// Extract the payload from the message handle with ::ovr_Message_GetPurchaseArray().
OVRP_PUBLIC_FUNCTION(ovrRequest) ovr_IAP_GetViewerPurchases();

/// Retrieve a list of Purchase that the Logged-In-User has made. This list
/// will only contain durable purchase (non-consumable) and is populated from a
/// device cache. It is recommended in all cases to use
/// ovr_User_GetViewerPurchases first and only check the cache if that fails.
///
/// A message with type ::ovrMessage_IAP_GetViewerPurchasesDurableCache will be generated in response.
///
/// First call ::ovr_Message_IsError() to check if an error occurred.
///
/// If no error occurred, the message will contain a payload of type ::ovrPurchaseArrayHandle.
/// Extract the payload from the message handle with ::ovr_Message_GetPurchaseArray().
OVRP_PUBLIC_FUNCTION(ovrRequest) ovr_IAP_GetViewerPurchasesDurableCache();

/// Launch the checkout flow to purchase the existing product. Oculus Home
/// tries handle and fix as many errors as possible. Home returns the
/// appropriate error message and how to resolveit, if possible. Returns a
/// purchase on success, empty purchase on cancel, and an error on error.
/// \param sku IAP sku for the item the user wishes to purchase.
///
/// A message with type ::ovrMessage_IAP_LaunchCheckoutFlow will be generated in response.
///
/// First call ::ovr_Message_IsError() to check if an error occurred.
///
/// If no error occurred, the message will contain a payload of type ::ovrPurchaseHandle.
/// Extract the payload from the message handle with ::ovr_Message_GetPurchase().
OVRP_PUBLIC_FUNCTION(ovrRequest) ovr_IAP_LaunchCheckoutFlow(const char *sku);

#endif
