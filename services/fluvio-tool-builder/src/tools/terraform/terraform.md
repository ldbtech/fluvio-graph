# Terraform Cloud Deployer Skill

## Purpose
Enables automatic deployment, scaling, and configuration of verified cloud topologies on AWS. It compiles and executes Terraform configurations representing pipelines, data warehouses, object stores, and computing layers.

## Supported Resources
- **VPC / Subnets**: Provision isolated networks, security groups, routing tables, and gateways.
- **S3 Bucket**: Setup cloud object storage with custom policies for CSV/Parquet dumps.
- **Redshift Cluster**: Spin up warehouse nodes to execute high-volume warehouse SQL analytics.
- **EC2 Instances**: Provision general purpose compute hosts for ETL runtimes or dbt scripts.
- **IAM Roles / Policies**: Setup secure access profiles for container pipelines.

## Common Operations
- **apply_infrastructure**: Initializes and executes `terraform apply` to provision AWS assets.
- **destroy_infrastructure**: Runs `terraform destroy` to cleanly tear down cloud sandboxes.
- **get_infrastructure_status**: Queries state mappings and public endpoints.
