#!/usr/bin/env bash

export ENV=$1
export APP=$2

if [ -z "$ENV" ];
then
  echo "Environment must be set!"
  exit 1
fi

if [ "$ENV" = "production" ]
then
  CLUSTER=production
else
  CLUSTER=staging
fi

if [ -z "$APP" ];
then
  echo "App name must be set!"
  exit 1
fi

build_docker(){
  sed -i.bk -e 's@{AWS_ACCOUNT_ID}@'"$AWS_ACCOUNT_ID"'@g' Dockerfile
  docker build -t $AWS_ACCOUNT_ID.dkr.ecr.eu-west-1.amazonaws.com/$APP:latest .
}

configure_aws_cli(){
	aws --version
	aws configure set default.region eu-west-1
	aws configure set default.output json
}

push_ecr_image(){
  eval $(aws ecr get-login --region eu-west-1 --no-include-email)
	docker tag $AWS_ACCOUNT_ID.dkr.ecr.eu-west-1.amazonaws.com/$APP:latest $AWS_ACCOUNT_ID.dkr.ecr.eu-west-1.amazonaws.com/$APP:$CIRCLE_BRANCH-$CIRCLE_BUILD_NUM
	docker push $AWS_ACCOUNT_ID.dkr.ecr.eu-west-1.amazonaws.com/$APP:latest
	docker push $AWS_ACCOUNT_ID.dkr.ecr.eu-west-1.amazonaws.com/$APP:$CIRCLE_BRANCH-$CIRCLE_BUILD_NUM
}

register_definition() {
	envsubst < deploy/task-definition.$ENV.json > task-definition.json
	revision=$(aws ecs register-task-definition --cli-input-json file://task-definition.json | jq --raw-output --exit-status '.taskDefinition.revision')
}

deploy_cluster(){
    aws ecs update-service --cluster $CLUSTER --service $APP --task-definition $APP-$ENV:$revision
}

build_docker
#configure_aws_cli
#push_ecr_image
#register_definition
#deploy_cluster

