# Namespace: conformance.enumattrs

## PaymentMethod ::enum
- cash: "현금"
- card: "카드"
- legacy_carryover: "레거시 이관 정리" @system

## ShippingPriority ::enum
- standard: integer = 0 "Standard Shipping"
- overnight: integer = 2 "Overnight Delivery" @deprecated("use express")

## Payment
- id: identifier @pk @generated
- method: PaymentMethod @not_null
- channel: enum = "web"
  - values:
    - web: "웹"
    - migration: "이관" @system
