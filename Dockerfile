FROM quay.io/operator-framework/ansible-operator:v1.28.1

USER root
RUN pip3 install psycopg2-binary==2.9.7 

ENV KUBECTL_VERSION=v1.28.0
RUN curl -LO https://dl.k8s.io/release/${KUBECTL_VERSION}/bin/linux/amd64/kubectl \
  && chmod +x kubectl && mv kubectl /usr/local/bin/ && kubectl version --client=true

USER 1001

COPY requirements.yml ${HOME}/requirements.yml
RUN ansible-galaxy collection install -r ${HOME}/requirements.yml \
 && chmod -R ug+rwx ${HOME}/.ansible

COPY watches.yaml ${HOME}/watches.yaml
COPY roles/ ${HOME}/roles/
COPY playbooks/ ${HOME}/playbooks/
