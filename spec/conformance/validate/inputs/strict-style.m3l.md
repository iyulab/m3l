# Namespace: conformance.strict

## Profile
- id: identifier @pk
- biography_summary_for_listing_pages: string(500) "A one-paragraph summary shown on listing pages"
- shipping: object
  - address: object
    - street: string(200)
  - method: string
- config: object
  - display: object
    - theme: object
      - colors: object
        - primary: string
