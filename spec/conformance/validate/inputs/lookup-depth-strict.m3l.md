# Namespace: conformance.validate

## Country
- id: identifier @pk
- name: string(100)

## Region
- id: identifier @pk
- country_id: identifier @reference(Country)

## City
- id: identifier @pk
- region_id: identifier @reference(Region)

## Site
- id: identifier @pk
- city_id: identifier @reference(City)

## Visit
- id: identifier @pk
- site_id: identifier @reference(Site)
- country_name: string @lookup(site_id.city_id.region_id.country_id.name)
