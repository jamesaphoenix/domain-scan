<?php

trait Timestampable {
    public function getCreatedAt(): \DateTime {
        return $this->createdAt;
    }

    public function setCreatedAt(\DateTime $date): void {
        $this->createdAt = $date;
    }
}

trait SoftDeletable {
    public function softDelete(): void {}

    public function restore(): void {}

    public function isDeleted(): bool {
        return false;
    }
}
