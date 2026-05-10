FROM python:3.12-alpine

WORKDIR /app
COPY requirements.txt .
RUN pip install --no-cache-dir -r requirements.txt

COPY *.py .

RUN adduser -D scanner
USER scanner

ENTRYPOINT ["python", "main.py"]
